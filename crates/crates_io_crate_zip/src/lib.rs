//! Builds a deterministic, seekable zip archive plus a JSON manifest from a
//! crate's `.crate` tarball.
//!
//! A `.crate` is a gzipped tarball: a single solid DEFLATE stream with no
//! random access to individual files. This crate re-packs the source files
//! into a `.zip`, which compresses each entry independently and therefore
//! supports fetching one file via a byte range. The accompanying [`Manifest`]
//! records, for each file, where its compressed payload starts in the zip and
//! the sha256 of its uncompressed contents, so a consumer can range-fetch and
//! verify a single file without parsing the zip central directory.

use anyhow::{Context, bail};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Seek, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// The timestamp type stamped on every zip entry, re-exported so callers can
/// build one without depending on the `zip` crate directly.
pub use zip::DateTime;

/// Name of the entry that is always written first in the zip.
const CARGO_TOML: &str = "Cargo.toml";

/// Chunk size used while streaming entry contents into the zip and hasher.
const CHUNK_SIZE: usize = 64 * 1024;

/// Describes the contents of a generated zip archive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    /// One entry per file in the zip, sorted alphabetically by path.
    pub files: Vec<FileEntry>,
}

/// A single file recorded in a [`Manifest`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    /// Realtive path (without the leading `{name}-{version}/` component of
    /// the tarball).
    pub path: String,
    /// Byte offset in the zip where this entry's compressed payload begins.
    pub data_offset: u64,
    /// Length of the compressed contents in bytes.
    pub compressed_size: u64,
    /// Length of the uncompressed contents in bytes.
    pub uncompressed_size: u64,
    /// How the payload is compressed: `"deflate"` or `"store"`.
    pub compression: String,
    /// Lowercase hex sha256 of the uncompressed contents.
    pub sha256: String,
}

/// Builds a deterministic zip from a `.crate` (gzipped tarball) and returns its
/// [`Manifest`].
pub fn build_zip<R: Read + Seek, W: Read + Write + Seek>(
    tarball: &mut R,
    modified: zip::DateTime,
    zip_out: W,
) -> anyhow::Result<Manifest> {
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(9))
        .last_modified_time(modified);

    let mut zip = ZipWriter::new(zip_out);

    // Pass 1: Count how often each path occurs, and buffer the contents of the
    // last `Cargo.toml`. We match case-insensitively to also catch the
    // deprecated lowercase `cargo.toml` and keep the last occurrence to
    // match cargo, which overwrites earlier entries when it extracts a crate.
    let mut remaining: HashMap<String, usize> = HashMap::new();
    let mut cargo_toml: Option<(String, Vec<u8>)> = None;
    for_each_file(tarball, |path, reader| {
        *remaining.entry(path.to_string()).or_insert(0) += 1;

        if path.eq_ignore_ascii_case(CARGO_TOML) {
            let mut contents = Vec::new();
            reader
                .read_to_end(&mut contents)
                .with_context(|| format!("Failed to read contents of `{path}`"))?;
            cargo_toml = Some((path.to_string(), contents));
        }

        Ok(())
    })?;

    // Maps each entry's path to the raw sha256 of its uncompressed contents.
    let mut hashes: HashMap<String, [u8; 32]> = HashMap::new();

    // Write `Cargo.toml` first so a consumer can fetch it from the start of the
    // zip without consulting the manifest.
    if let Some((path, contents)) = &cargo_toml {
        let sha256 = write_entry(&mut zip, path, options, &mut contents.as_slice())?;
        hashes.insert(path.clone(), sha256);
    }

    // Pass 2: Write the last occurrence of every other path. A `.crate` may
    // carry several entries with the same path, which cargo's extraction
    // resolves by overwriting on disk (the last entry wins). The zip format
    // cannot hold duplicate names, so we keep only the last occurrence to match
    // that behavior.
    for_each_file(tarball, |path, reader| {
        if path.eq_ignore_ascii_case(CARGO_TOML) {
            return Ok(());
        }

        if let Some(count) = remaining.get_mut(path) {
            *count -= 1;
            if *count == 0 {
                let sha256 = write_entry(&mut zip, path, options, reader)?;
                hashes.insert(path.to_string(), sha256);
            }
        }

        Ok(())
    })?;

    let mut archive = zip
        .finish_into_readable()
        .context("Failed to finalize zip archive")?;

    let mut files = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        // `_raw` because we only read each entry's metadata, never its bytes,
        // so there is no need to set up a decompressor.
        let entry = archive
            .by_index_raw(i)
            .context("Failed to read zip entry")?;

        let path = entry.name().to_string();

        let data_offset = entry
            .data_start()
            .with_context(|| format!("Missing data offset for zip entry `{path}`"))?;

        let compression = match entry.compression() {
            CompressionMethod::Deflated => "deflate",
            CompressionMethod::Stored => "store",
            other => bail!("Unexpected compression method `{other}` for zip entry `{path}`"),
        };

        let sha256 = hashes
            .get(&path)
            .with_context(|| format!("Missing sha256 for zip entry `{path}`"))?;

        files.push(FileEntry {
            data_offset,
            compressed_size: entry.compressed_size(),
            uncompressed_size: entry.size(),
            compression: compression.to_string(),
            sha256: hex::encode(sha256),
            path,
        });
    }

    // Order the manifest alphabetically (case-insensitive) by path.
    files.sort_by_cached_key(|f| (f.path.to_lowercase(), f.path.clone()));

    Ok(Manifest { files })
}

/// Decodes the gzipped tarball and invokes `callback` for each regular file
/// entry, passing its crate-root-relative path and a reader over its contents.
/// `tarball` is rewound before decoding, so it can be called repeatedly on the
/// same reader.
fn for_each_file<R: Read + Seek>(
    tarball: &mut R,
    mut callback: impl FnMut(&str, &mut dyn Read) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    tarball.rewind().context("Failed to rewind tarball")?;

    let decoder = GzDecoder::new(tarball);
    let mut archive = tar::Archive::new(decoder);

    let entries = archive
        .entries()
        .context("Failed to read tarball entries")?;

    for entry in entries {
        let mut entry = entry.context("Failed to read tarball entry")?;
        if !entry.header().entry_type().is_file() {
            // Skip directories, symlinks, hardlinks, and other non-file entries.
            continue;
        }

        let path = entry.path().context("Failed to read tarball entry path")?;
        let path = path
            .to_str()
            .context("Tarball entry path is not valid UTF-8")?;

        // Strip the leading `{name}-{version}/` component so zip paths are
        // crate-root-relative. The prefix is whatever the first component is,
        // rather than a hardcoded name. Publish validation guarantees every
        // entry sits under that directory, so an entry without one means a
        // malformed `.crate`.
        let stripped = match path.split_once('/') {
            Some((_, rest)) => rest.to_string(),
            None => bail!("Tarball entry `{path}` is not inside the package directory"),
        };

        callback(&stripped, &mut entry)?;
    }

    Ok(())
}

/// Streams one entry into the zip, computing the sha256 of its uncompressed
/// contents as the bytes flow through. Returns the raw 32-byte digest.
fn write_entry<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    path: &str,
    options: SimpleFileOptions,
    reader: &mut dyn Read,
) -> anyhow::Result<[u8; 32]> {
    zip.start_file(path, options)
        .with_context(|| format!("Failed to start zip entry `{path}`"))?;

    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHUNK_SIZE];
    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("Failed to read contents of `{path}`"))?;

        if read == 0 {
            break;
        }

        let chunk = &buffer[..read];
        hasher.update(chunk);
        zip.write_all(chunk)
            .with_context(|| format!("Failed to write zip entry `{path}`"))?;
    }

    Ok(hasher.finalize().into())
}
