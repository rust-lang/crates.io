#![doc = include_str!("../README.md")]

#[cfg(test)]
#[macro_use]
extern crate claims;

#[cfg(any(feature = "builder", test))]
pub use crate::builder::TarballBuilder;
use crate::limit_reader::LimitErrorReader;
use crate::manifest::validate_manifest;
pub use crate::vcs_info::CargoVcsInfo;
use crates_io_cargo_toml::AbstractFilesystem;
pub use crates_io_cargo_toml::{Manifest, StringOrBool};
use futures_util::StreamExt;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use tokio::io::{AsyncReadExt, BufReader};
use tokio_tar::{Entry, EntryType};
use tracing::{instrument, warn};

#[cfg(any(feature = "builder", test))]
mod builder;
mod limit_reader;
mod manifest;
mod vcs_info;

const DEFAULT_BUF_SIZE: usize = 128 * 1024;

/// Resource limits applied while processing a crate tarball.
#[derive(Clone, Copy, Debug)]
pub struct TarballLimits {
    /// Maximum size in bytes of a crate tarball once decompressed.
    pub unpack_size: u64,
    /// Maximum number of archive entries, including directory entries.
    ///
    /// Metadata headers consumed by the tar parser are not counted. `None`
    /// disables the limit.
    pub entries: Option<usize>,
}

#[derive(Debug)]
pub struct TarballInfo {
    pub manifest: Manifest,
    pub vcs_info: Option<CargoVcsInfo>,
}

#[derive(Debug, thiserror::Error)]
pub enum TarballError {
    #[error("uploaded tarball is malformed or too large when decompressed")]
    Malformed(#[source] std::io::Error),
    #[error("invalid path found: {0}")]
    InvalidPath(String),
    #[error("malformed pax size")]
    MalformedPaxSize,
    #[error("mismatched pax and tar header sizes")]
    SizeMismatch,
    #[error("unexpected tar entry type {entry_type:?} found: {path}")]
    UnexpectedEntry { path: String, entry_type: EntryType },
    #[error("uploaded tarball contains more than {max} entries")]
    TooManyEntries { max: usize },
    #[error("Cargo.toml manifest is missing")]
    MissingManifest,
    #[error("Cargo.toml manifest is invalid: {0}")]
    InvalidManifest(#[from] crates_io_cargo_toml::Error),
    #[error("Cargo.toml manifest is incorrectly cased: {0:?}")]
    IncorrectlyCasedManifest(PathBuf),
    #[error("more than one Cargo.toml manifest in tarball: {0:?}")]
    TooManyManifests(Vec<PathBuf>),
    #[error(transparent)]
    IO(#[from] std::io::Error),
}

#[instrument(skip_all, fields(%pkg_name))]
pub async fn process_tarball<R: tokio::io::AsyncRead + Unpin>(
    pkg_name: &str,
    tarball: R,
    limits: TarballLimits,
) -> Result<TarballInfo, TarballError> {
    let tarball = BufReader::with_capacity(DEFAULT_BUF_SIZE, tarball);
    // All our data is currently encoded with gzip
    let decoder = async_compression::tokio::bufread::GzipDecoder::new(tarball);

    // Don't let gzip decompression go into the weeeds, apply a fixed cap after
    // which point we say the decompressed source is "too large".
    let decoder = LimitErrorReader::new(decoder, limits.unpack_size);

    // Use this I/O object now to take a peek inside
    let mut archive = tokio_tar::Archive::new(decoder);

    let pkg_root = Path::new(&pkg_name);

    let mut vcs_info = None;
    let mut paths = Vec::new();
    let mut manifests = BTreeMap::new();
    let mut entries = archive.entries()?;
    let mut num_entries = 0;

    while let Some(entry) = entries.next().await {
        let mut entry = entry.map_err(TarballError::Malformed)?;

        if let Some(max) = limits.entries {
            num_entries += 1;
            if num_entries > max {
                return Err(TarballError::TooManyEntries { max });
            }
        }

        // Check that the file size is consistent between the pax and tar
        // headers. We have to do this before anything else because iterating
        // the pax headers requires a mutable reference to entry.
        validate_pax_size(&mut entry).await.inspect_err(|e| {
            warn!(%e, ?entry, pkg_name, "file size validation failure");
        })?;

        // Verify that all entries actually start with `$name-$vers/`.
        // Historically Cargo didn't verify this on extraction so you could
        // upload a tarball that contains both `foo-0.1.0/` source code as well
        // as `bar-0.1.0/` source code, and this could overwrite other crates in
        // the registry!
        let entry_path = entry.path()?;
        let Ok(in_pkg_path) = entry_path.strip_prefix(pkg_root) else {
            return Err(TarballError::InvalidPath(entry_path.display().to_string()));
        };

        // Reject any paths that contain `..` components. This is a security
        // measure to prevent directory traversal attacks, where an attacker
        // could craft a tarball that extracts files outside the
        // intended directory.
        if in_pkg_path
            .components()
            .any(|component| component == Component::ParentDir)
        {
            return Err(TarballError::InvalidPath(entry_path.display().to_string()));
        }

        // Reject any paths that are not UTF-8.
        let Some(in_pkg_path_str) = in_pkg_path.to_str() else {
            return Err(TarballError::InvalidPath(entry_path.display().to_string()));
        };

        // Crate packages only need regular files and directories. Other entry
        // types have special or implementation-dependent extraction behavior.
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err(TarballError::UnexpectedEntry {
                path: entry_path.display().to_string(),
                entry_type,
            });
        }

        paths.push(in_pkg_path.to_path_buf());

        // Let's go hunting for the VCS info and crate manifest. The only valid place for these is
        // in the package root in the tarball.
        if in_pkg_path_str == ".cargo_vcs_info.json" {
            let mut contents = String::new();
            entry.read_to_string(&mut contents).await?;
            vcs_info = CargoVcsInfo::from_contents(&contents).ok();
        } else if in_pkg_path_str.eq_ignore_ascii_case("cargo.toml") {
            // Try to extract and read the Cargo.toml from the tarball, silently erroring if it
            // cannot be read.
            let owned_entry_path = entry_path.into_owned();
            let mut contents = String::new();
            entry.read_to_string(&mut contents).await?;

            let manifest = Manifest::from_str(&contents)?;
            validate_manifest(&manifest)?;

            manifests.insert(owned_entry_path, manifest);
        }
    }

    if manifests.len() > 1 {
        // There are no scenarios where we want to accept a crate file with multiple manifests.
        return Err(TarballError::TooManyManifests(
            manifests.into_keys().collect(),
        ));
    }

    // Although we're interested in all possible cases of `Cargo.toml` above to protect users
    // on case-insensitive filesystems, to match the behaviour of cargo we should only actually
    // accept `Cargo.toml` and (the now deprecated) `cargo.toml` as valid options for the
    // manifest.
    let Some((path, mut manifest)) = manifests.pop_first() else {
        return Err(TarballError::MissingManifest);
    };

    let file = path.file_name().unwrap_or_default();
    if file != "Cargo.toml" && file != "cargo.toml" {
        return Err(TarballError::IncorrectlyCasedManifest(file.into()));
    }

    manifest.complete_from_abstract_filesystem(&PathsFileSystem(paths))?;

    Ok(TarballInfo { manifest, vcs_info })
}

async fn validate_pax_size<R: tokio::io::AsyncRead + Unpin>(
    entry: &mut Entry<R>,
) -> Result<(), TarballError> {
    // Ensure that, if we have both tar and pax header file sizes, they match so
    // that downstream users of crates cannot be subjected to confusion attacks
    // if they prioritise those headers differently when deciding on the source
    // of truth.
    //
    // It's not an error to not have a pax header — while Cargo will always
    // include them, as will pretty much every modern tar implementation, it's
    // really not an issue if it's not there. It's just an issue if it doesn't
    // match.
    //
    // Note that this implies the files cannot be larger than the limit in
    // legacy tar headers, which is 8 GiB. In practice, this should not be an
    // issue for crates.io given our other limits.

    let tar_size = entry.header().size().map_err(TarballError::Malformed)?;

    if let Some(pax) = entry.pax_extensions().await? {
        for ext_result in pax {
            let ext = ext_result.map_err(TarballError::Malformed)?;
            if ext.key().is_ok_and(|key| key == "size") {
                let pax_size = ext
                    .value()
                    .map_err(|_| TarballError::MalformedPaxSize)?
                    .parse::<u64>()
                    .map_err(|_| TarballError::MalformedPaxSize)?;

                if pax_size != tar_size {
                    return Err(TarballError::SizeMismatch);
                }
            }
        }
    }

    Ok(())
}

struct PathsFileSystem(Vec<PathBuf>);

impl AbstractFilesystem for PathsFileSystem {
    fn file_names_in<T: AsRef<Path>>(&self, rel_path: T) -> std::io::Result<BTreeSet<Box<str>>> {
        let mut rel_path = rel_path.as_ref();

        // Deal with relative paths that start with `./`
        let mut components = rel_path.components();
        while components.next() == Some(Component::CurDir) {
            rel_path = components.as_path();
        }

        let paths = &self.0;
        let file_names = paths
            .iter()
            .filter_map(move |p| p.strip_prefix(rel_path).ok())
            .filter_map(|name| match name.components().next() {
                // `process_tarball()` rejects non-utf8 paths before they reach here, so `to_str()` should always succeeds
                Some(Component::Normal(p)) => p.to_str(),
                _ => None,
            })
            .map(From::from)
            .collect();

        Ok(file_names)
    }
}

#[cfg(test)]
mod tests {
    use super::{TarballLimits, process_tarball};
    use crate::TarballBuilder;
    use insta::{assert_debug_snapshot, assert_snapshot};

    const MANIFEST: &[u8] = b"[package]\nname = \"foo\"\nversion = \"0.0.1\"\n";
    const LIMITS: TarballLimits = TarballLimits {
        unpack_size: 512 * 1024 * 1024,
        entries: None,
    };

    fn tarball_with_entry_type(entry_type: tar::EntryType) -> Vec<u8> {
        let mut builder = TarballBuilder::new().add_file("foo-0.0.1/Cargo.toml", MANIFEST);

        let mut header = tar::Header::new_gnu();
        header.set_path("foo-0.0.1/bar").unwrap();
        header.set_size(0);
        header.set_entry_type(entry_type);
        if entry_type.is_hard_link() || entry_type.is_symlink() {
            header.set_link_name("foo-0.0.1/target").unwrap();
        }
        if entry_type.is_gnu_sparse() {
            header.as_gnu_mut().unwrap().set_real_size(0);
        }
        header.set_cksum();
        builder.as_mut().append(&header, &[][..]).unwrap();

        builder.build()
    }

    #[tokio::test]
    async fn process_tarball_test() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_dir("foo-0.0.1")
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_debug_snapshot!(tarball_info);

        let err = assert_err!(process_tarball("bar-0.0.1", &*tarball, LIMITS).await);
        assert_snapshot!(err, @"invalid path found: foo-0.0.1/Cargo.toml");
    }

    #[tokio::test]
    async fn process_tarball_test_unexpected_entry_types() {
        let unexpected_entry_types = [
            (tar::EntryType::new(b'Z'), "Other(90)"),
            (tar::EntryType::contiguous(), "Continuous"),
            (tar::EntryType::new(b'S'), "GNUSparse"),
            (tar::EntryType::hard_link(), "Link"),
            (tar::EntryType::symlink(), "Symlink"),
            (tar::EntryType::character_special(), "Char"),
            (tar::EntryType::block_special(), "Block"),
            (tar::EntryType::fifo(), "Fifo"),
        ];

        for (entry_type, entry_type_name) in unexpected_entry_types {
            let tarball = tarball_with_entry_type(entry_type);
            let error = assert_err!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
            assert_eq!(
                error.to_string(),
                format!("unexpected tar entry type {entry_type_name} found: foo-0.0.1/bar")
            );
        }
    }

    #[tokio::test]
    async fn process_tarball_test_gnu_long_name() {
        let long_path = format!("foo-0.0.1/{}", "a".repeat(100));
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file(&long_path, b"")
            .build();

        assert_ok!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
    }

    #[tokio::test]
    async fn process_tarball_test_parent_component() {
        let mut builder = TarballBuilder::new().add_file("foo-0.0.1/Cargo.toml", MANIFEST);

        let mut header = tar::Header::new_gnu();
        let path = b"foo-0.0.1/../outside";
        // `tar::Header::set_path()` rejects parent components, so construct the
        // untrusted path from raw bytes.
        header.as_mut_bytes()[..path.len()].copy_from_slice(path);
        header.set_size(0);
        header.set_cksum();
        builder.as_mut().append(&header, b"".as_slice()).unwrap();

        let tarball = builder.build();
        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_snapshot!(err, @"invalid path found: foo-0.0.1/../outside");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn process_tarball_test_non_utf8_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let mut builder = TarballBuilder::new().add_file("foo-0.0.1/Cargo.toml", MANIFEST);

        let mut header = tar::Header::new_gnu();
        header
            .set_path(OsStr::from_bytes(b"foo-0.0.1/\xff"))
            .unwrap();
        header.set_size(0);
        header.set_cksum();
        builder.as_mut().append(&header, b"".as_slice()).unwrap();

        let tarball = builder.build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_snapshot!(err, @"invalid path found: foo-0.0.1/�");
    }

    #[tokio::test]
    async fn process_tarball_test_size_limit() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let limits = TarballLimits {
            unpack_size: tarball.len() as u64 - 1,
            entries: None,
        };
        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, limits).await);
        assert_snapshot!(err, @"uploaded tarball is malformed or too large when decompressed");
    }

    #[tokio::test]
    async fn process_tarball_test_entry_limit() {
        let tarball = TarballBuilder::new()
            .add_pax_extensions([("comment", b"metadata".as_slice())])
            .add_dir("foo-0.0.1")
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_dir("foo-0.0.1/src")
            .add_file("foo-0.0.1/src/lib.rs", b"pub fn foo() {}")
            .build();

        let limits = TarballLimits {
            entries: Some(4),
            ..LIMITS
        };
        assert_ok!(process_tarball("foo-0.0.1", &*tarball, limits).await);

        let limits = TarballLimits {
            entries: Some(3),
            ..LIMITS
        };
        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, limits).await);
        assert_snapshot!(err, @"uploaded tarball contains more than 3 entries");
    }

    #[tokio::test]
    async fn process_tarball_test_incomplete_vcs_info() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/.cargo_vcs_info.json", br#"{"unknown": "field"}"#)
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_some!(&tarball_info.vcs_info);
        assert_debug_snapshot!(tarball_info);
    }

    #[tokio::test]
    async fn process_tarball_test_vcs_info() {
        let vcs_info = br#"{"path_in_vcs": "path/in/vcs"}"#;
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/.cargo_vcs_info.json", vcs_info)
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_some!(&tarball_info.vcs_info);
        assert_debug_snapshot!(tarball_info);
    }

    #[tokio::test]
    async fn process_tarball_test_manifest() {
        let manifest = br#"
            [package]
            name = "foo"
            version = "0.0.1"
            rust-version = "1.59"
            readme = "README.md"
            repository = "https://github.com/foo/bar"
            "#;
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", manifest)
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_debug_snapshot!(tarball_info);
    }

    #[tokio::test]
    async fn process_tarball_test_manifest_with_project() {
        let manifest = br#"
            [project]
            name = "foo"
            version = "0.0.1"
            rust-version = "1.23"
            "#;
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", manifest)
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_debug_snapshot!(tarball_info);
    }

    #[tokio::test]
    async fn process_tarball_test_manifest_with_default_readme() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_debug_snapshot!(tarball_info);
    }

    #[tokio::test]
    async fn process_tarball_test_manifest_with_boolean_readme() {
        let manifest = br#"
            [package]
            name = "foo"
            version = "0.0.1"
            readme = false
            "#;
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", manifest)
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_debug_snapshot!(tarball_info);
    }

    #[tokio::test]
    async fn process_tarball_test_lowercase_manifest() {
        let manifest = br#"
            [package]
            name = "foo"
            version = "0.0.1"
            repository = "https://github.com/foo/bar"
            "#;
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/cargo.toml", manifest)
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_debug_snapshot!(tarball_info);
    }

    #[tokio::test]
    async fn process_tarball_test_incorrect_manifest_casing() {
        let process = async |file| {
            let tarball = TarballBuilder::new()
                .add_file(&format!("foo-0.0.1/{file}"), MANIFEST)
                .build();

            process_tarball("foo-0.0.1", &*tarball, LIMITS).await
        };

        let err = assert_err!(process("CARGO.TOML").await);
        assert_snapshot!(err, @r#"Cargo.toml manifest is incorrectly cased: "CARGO.TOML""#);

        let err = assert_err!(process("Cargo.Toml").await);
        assert_snapshot!(err, @r#"Cargo.toml manifest is incorrectly cased: "Cargo.Toml""#);
    }

    #[tokio::test]
    async fn process_tarball_test_multiple_manifests() {
        let process = async |files: Vec<_>| {
            let tarball = files
                .iter()
                .fold(TarballBuilder::new(), |builder, file| {
                    builder.add_file(&format!("foo-0.0.1/{file}"), MANIFEST)
                })
                .build();

            process_tarball("foo-0.0.1", &*tarball, LIMITS).await
        };

        let err = assert_err!(process(vec!["cargo.toml", "Cargo.toml"]).await);
        assert_snapshot!(err, @r#"more than one Cargo.toml manifest in tarball: ["foo-0.0.1/Cargo.toml", "foo-0.0.1/cargo.toml"]"#);

        let err = assert_err!(process(vec!["Cargo.toml", "Cargo.Toml"]).await);
        assert_snapshot!(err, @r#"more than one Cargo.toml manifest in tarball: ["foo-0.0.1/Cargo.Toml", "foo-0.0.1/Cargo.toml"]"#);

        let err = assert_err!(process(vec!["Cargo.toml", "cargo.toml", "CARGO.TOML"]).await);
        assert_snapshot!(err, @r#"more than one Cargo.toml manifest in tarball: ["foo-0.0.1/CARGO.TOML", "foo-0.0.1/Cargo.toml", "foo-0.0.1/cargo.toml"]"#);
    }

    #[tokio::test]
    async fn process_tarball_test_size_malformed() {
        // There are two cases here: one where the size value is invalid UTF-8,
        // and another where it's just not a number.
        //
        // We have to build this with the regular `tar` crate.

        let tarball = TarballBuilder::new()
            .add_pax_extensions([("size", b"\xff\xfe".as_slice())])
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_snapshot!(err, @"uploaded tarball is malformed or too large when decompressed");

        let tarball = TarballBuilder::new()
            .add_pax_extensions([("size", b"not-a-valid-number".as_slice())])
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_snapshot!(err, @"uploaded tarball is malformed or too large when decompressed");
    }

    #[tokio::test]
    async fn process_tarball_test_size_mismatch() {
        // Build up a tarball with a mismatch between its pax and tar header
        // sizes.
        let tarball = TarballBuilder::new()
            // Set the pax size to a larger value.
            .add_pax_extensions([("size", "2048".as_bytes())])
            // Add the real content, which is less than a single tar block (512
            // bytes).
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            // Add a symlink in the overlap.
            .add_symlink("smuggled", "/etc/issue")
            .build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_snapshot!(err, @"mismatched pax and tar header sizes");
    }

    #[tokio::test]
    async fn test_lib() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/src/lib.rs", b"pub fn foo() {}")
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_debug_snapshot!(tarball_info);
    }

    #[tokio::test]
    async fn test_lib_with_bins_and_example() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/examples/how-to-use-foo.rs", b"fn main() {}")
            .add_file("foo-0.0.1/src/lib.rs", b"pub fn foo() {}")
            .add_file("foo-0.0.1/src/bin/foo.rs", b"fn main() {}")
            .add_file("foo-0.0.1/src/bin/bar.rs", b"fn main() {}")
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_debug_snapshot!(tarball_info);
    }

    #[tokio::test]
    async fn test_app() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/src/main.rs", b"fn main() {}")
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, LIMITS).await);
        assert_debug_snapshot!(tarball_info);
    }
}
