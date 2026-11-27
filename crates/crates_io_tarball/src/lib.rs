#![doc = include_str!("../README.md")]

#[cfg(test)]
#[macro_use]
extern crate claims;

#[cfg(any(feature = "builder", test))]
pub use crate::builder::TarballBuilder;
use crate::last_block_reader::{LastBlock, LastBlockReader};
use crate::limit_reader::LimitErrorReader;
use crate::manifest::validate_manifest;
pub use crate::vcs_info::CargoVcsInfo;
use crates_io_cargo_toml::target_metadata::{self, TargetMetadata};
pub use crates_io_cargo_toml::{Manifest, StringOrBool};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use tar_codec::{
    Archive, DecodeError, DecodePolicy, Member, MemberPayload, PaxDecodePolicy,
    PaxVendorExtensionPolicy, SpecialKind, TarArchive, TrailingDataPolicy,
};
use tar_framing::header::{IDENTITY_RANGE, SIZE_RANGE, USTAR_IDENTITY};
use tar_framing::{FrameError, FrameErrorInner};
use tokio::io::{AsyncReadExt, BufReader};
use tracing::{instrument, warn};

#[cfg(any(feature = "builder", test))]
mod builder;
mod last_block_reader;
mod limit_reader;
mod manifest;
mod vcs_info;

const DEFAULT_BUF_SIZE: usize = 128 * 1024;

/// Validation rules and resource limits applied while processing a crate tarball.
#[derive(Clone, Copy, Debug)]
pub struct TarballPolicy {
    /// Maximum size in bytes of a crate tarball once decompressed.
    pub unpack_size: u64,
    /// Maximum number of archive entries, including directory entries.
    ///
    /// Metadata headers consumed by the tar parser are not counted. `None`
    /// disables the limit.
    pub entries: Option<usize>,
    /// Maximum size in bytes of the `Cargo.toml` and `.cargo_vcs_info.json`
    /// entries.
    pub metadata_file_size: u64,
    pub compatibility: TarballCompatibility,
}

impl TarballPolicy {
    /// Returns a rule violation in strict mode, or logs it in legacy mode.
    fn enforce_rule(&self, pkg_name: &str, error: TarballError) -> Result<(), TarballError> {
        match self.compatibility {
            TarballCompatibility::Strict => Err(error),
            TarballCompatibility::Legacy => {
                warn!("Accepting tarball `{pkg_name}` despite validation error: {error}");
                Ok(())
            }
        }
    }
}

/// How validation rule violations are handled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TarballCompatibility {
    /// Rejects the tarball. Used for new publishes.
    Strict,
    /// Logs a warning and accepts the tarball. Used for crates that were
    /// published before the rule existed.
    Legacy,
}

impl TarballCompatibility {
    /// Returns the `tar-codec` decoding rules for this mode.
    ///
    /// Strict mode uses the `tar-codec` defaults and also rejects global pax
    /// headers. Legacy mode accepts quirks of older tar writers, like
    /// unterminated names, base-256 IDs, unknown pax vendor records and data
    /// after the end-of-archive marker. Neither mode accepts repeated pax
    /// keywords, which readers resolve differently.
    pub fn decode_policy(self) -> DecodePolicy {
        match self {
            Self::Strict => {
                let pax_policy = PaxDecodePolicy::default().allow_global_pax_extensions(false);
                DecodePolicy::default().pax_policy(pax_policy)
            }
            Self::Legacy => {
                let pax_policy = PaxDecodePolicy::default()
                    .vendor_extension_policy(PaxVendorExtensionPolicy::AllowUnknown)
                    .allow_base256_uid_and_gid(true);

                DecodePolicy::default()
                    .trailing_data_policy(TrailingDataPolicy::Ignore)
                    .allow_header_uname_and_gname_without_terminator(true)
                    .allow_gnu_extensions_without_terminator(true)
                    .pax_policy(pax_policy)
            }
        }
    }
}

#[derive(Debug)]
pub struct TarballInfo {
    pub manifest: Manifest,
    /// Source entry points derived from the manifest and tarball contents.
    pub target_metadata: TargetMetadata,
    pub vcs_info: Option<CargoVcsInfo>,
}

#[derive(Debug, thiserror::Error)]
pub enum TarballError {
    #[error("uploaded tarball is malformed or too large when decompressed")]
    Malformed(#[source] std::io::Error),
    #[error("invalid path found: {0}")]
    InvalidPath(String),
    #[error("uploaded tarball is invalid: {0}")]
    Decode(DecodeError),
    #[error(
        "uploaded tarball is invalid: at byte {position}: pax size {pax_size} does not match header size {header_size}"
    )]
    SizeMismatch {
        position: u64,
        pax_size: u64,
        header_size: u64,
    },
    #[error("unexpected tar entry type {entry_type} found: {path}")]
    UnexpectedEntry {
        path: String,
        entry_type: &'static str,
    },
    #[error("uploaded tarball contains data after the end of the gzip stream")]
    TrailingData,
    #[error("uploaded tarball contains more than {max} entries")]
    TooManyEntries { max: usize },
    #[error("metadata file `{path}` exceeds the maximum size of {max} bytes")]
    MetadataFileTooLarge { path: String, max: u64 },
    #[error("Cargo.toml manifest is missing")]
    MissingManifest,
    #[error("Cargo.toml manifest is invalid: {0}")]
    InvalidManifest(#[from] crates_io_cargo_toml::Error),
    #[error("Cargo.toml target metadata is invalid: {0}")]
    InvalidTargetMetadata(#[source] target_metadata::Error),
    #[error("Cargo.toml manifest is incorrectly cased: {0:?}")]
    IncorrectlyCasedManifest(PathBuf),
    #[error("more than one Cargo.toml manifest in tarball: {0:?}")]
    TooManyManifests(Vec<PathBuf>),
    #[error(transparent)]
    IO(#[from] std::io::Error),
}

impl TarballError {
    fn unexpected_entry(path: &Path, entry_type: &'static str) -> Self {
        let path = path.display().to_string();
        Self::UnexpectedEntry { path, entry_type }
    }
}

impl From<DecodeError> for TarballError {
    fn from(error: DecodeError) -> Self {
        match error {
            DecodeError::Framing(FrameError {
                inner: FrameErrorInner::Io { source },
                ..
            }) => Self::Malformed(source),
            error => Self::Decode(error),
        }
    }
}

#[instrument(skip_all, fields(%pkg_name))]
pub async fn process_tarball<R: tokio::io::AsyncRead + Unpin>(
    pkg_name: &str,
    tarball: R,
    policy: TarballPolicy,
) -> Result<TarballInfo, TarballError> {
    let tarball = BufReader::with_capacity(DEFAULT_BUF_SIZE, tarball);
    // All our data is currently encoded with gzip
    let decoder = async_compression::tokio::bufread::GzipDecoder::new(tarball);

    // Don't let gzip decompression go into the weeeds, apply a fixed cap after
    // which point we say the decompressed source is "too large".
    let mut decoder = LimitErrorReader::new(decoder, policy.unpack_size);

    let (reader, last_block) = LastBlockReader::new(&mut decoder);
    let mut archive = TarArchive::new(reader).with_policy(policy.compatibility.decode_policy());

    let pkg_root = Path::new(&pkg_name);

    let mut vcs_info = None;
    let mut files = Vec::new();
    let mut manifests = BTreeMap::new();
    let mut num_entries = 0;

    while let Some(member) = archive.next_member().await? {
        if let Some(max) = policy.entries {
            num_entries += 1;
            if num_entries > max {
                return Err(TarballError::TooManyEntries { max });
            }
        }

        // Verify that all entries actually start with `$name-$vers/`.
        // Historically Cargo didn't verify this on extraction so you could
        // upload a tarball that contains both `foo-0.1.0/` source code as well
        // as `bar-0.1.0/` source code, and this could overwrite other crates in
        // the registry!
        let entry_path = PathBuf::from(&member.metadata().path);
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

        if policy.compatibility == TarballCompatibility::Strict
            && let Member::File { size, .. } = &member
        {
            check_header_size(&last_block, member.metadata().position, *size)?;
        }

        // Crate packages only need regular files and directories. Other entry
        // types have special or implementation-dependent extraction behavior.
        let (size, payload) = match member {
            Member::File { size, payload, .. } => (size, payload),
            Member::Directory { .. } => continue,
            Member::SymbolicLink { .. } => {
                return Err(TarballError::unexpected_entry(&entry_path, "Symlink"));
            }
            Member::HardLink { .. } => {
                return Err(TarballError::unexpected_entry(&entry_path, "Link"));
            }
            Member::Special { kind, .. } => {
                let entry_type = match kind {
                    SpecialKind::CharacterDevice => "Char",
                    SpecialKind::BlockDevice => "Block",
                    SpecialKind::Fifo => "Fifo",
                };
                return Err(TarballError::unexpected_entry(&entry_path, entry_type));
            }
        };

        files.push(in_pkg_path.to_path_buf());

        // Let's go hunting for the VCS info and crate manifest. The only valid place for these is
        // in the package root in the tarball.
        let max_size = policy.metadata_file_size;
        if in_pkg_path == Path::new(".cargo_vcs_info.json") {
            let contents = read_metadata_payload(payload, size, max_size, &entry_path).await?;
            vcs_info = CargoVcsInfo::from_contents(&contents).ok();
        } else if in_pkg_path.as_os_str().eq_ignore_ascii_case("cargo.toml") {
            // Try to extract and read the Cargo.toml from the tarball, silently erroring if it
            // cannot be read.
            let contents = read_metadata_payload(payload, size, max_size, &entry_path).await?;

            let manifest = Manifest::from_str(&contents)?;
            validate_manifest(&manifest)?;

            manifests.insert(entry_path, manifest);
        }
    }

    // In legacy mode, the tar decoder stops at the end-of-archive marker, so
    // the rest of the gzip member has not been read yet.
    drop(archive);
    tokio::io::copy(&mut decoder, &mut tokio::io::sink())
        .await
        .map_err(TarballError::Malformed)?;

    // The gzip decoder stops after the first member, but tools like `gzip`
    // and `bsdtar` also decode any members that follow it.
    let mut tarball = decoder.into_inner().into_inner();
    let mut next_byte = [0];
    let has_trailing_data = tarball.read(&mut next_byte).await? > 0;
    if has_trailing_data {
        policy.enforce_rule(pkg_name, TarballError::TrailingData)?;
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

    let target_metadata =
        TargetMetadata::extract(&mut manifest, files).map_err(|error| match error {
            target_metadata::Error::Manifest(error) => TarballError::InvalidManifest(error),
            error => TarballError::InvalidTargetMetadata(error),
        })?;

    Ok(TarballInfo {
        manifest,
        target_metadata,
        vcs_info,
    })
}

/// Rejects a file whose pax `size` record differs from its header `size`
/// field.
///
/// See the [`last_block_reader`] module for why this check is needed and how
/// it finds the header.
fn check_header_size(last_block: &LastBlock, position: u64, size: u64) -> Result<(), TarballError> {
    let Some(header) = last_block.get(position) else {
        let message = format!("tar header at byte {position} is not the last block read");
        return Err(std::io::Error::other(message).into());
    };

    // GNU headers cannot be combined with pax records.
    if header[IDENTITY_RANGE] != *USTAR_IDENTITY {
        return Ok(());
    }

    let digits = std::str::from_utf8(&header[SIZE_RANGE]).unwrap_or_default();
    let Ok(header_size) = u64::from_str_radix(digits.trim_end_matches(['\0', ' ']), 8) else {
        let message = format!("tar header at byte {position} has an invalid size field");
        return Err(std::io::Error::other(message).into());
    };

    if header_size != size {
        return Err(TarballError::SizeMismatch {
            position,
            pax_size: size,
            header_size,
        });
    }

    Ok(())
}

/// Reads a metadata file payload into memory after checking its declared size.
async fn read_metadata_payload<P: MemberPayload<Error = DecodeError>>(
    payload: P,
    size: u64,
    max: u64,
    path: &Path,
) -> Result<String, TarballError> {
    if size > max {
        let path = path.display().to_string();
        return Err(TarballError::MetadataFileTooLarge { path, max });
    }

    let contents = read_payload(payload).await?;
    String::from_utf8(contents)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error).into())
}

/// Reads the remaining payload of a tar member into memory.
pub async fn read_payload<P: MemberPayload>(mut payload: P) -> Result<Vec<u8>, P::Error> {
    let mut contents = Vec::new();
    let mut chunk = Vec::new();
    while payload.next_chunk(&mut chunk, DEFAULT_BUF_SIZE).await? {
        contents.extend_from_slice(&chunk);
    }
    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::{TarballCompatibility, TarballError, TarballInfo, TarballPolicy, process_tarball};
    use crate::TarballBuilder;
    use insta::{assert_debug_snapshot, assert_snapshot};
    use std::io::Read;

    const MANIFEST: &[u8] = b"[package]\nname = \"foo\"\nversion = \"0.0.1\"\n";
    const STRICT: TarballPolicy = TarballPolicy {
        unpack_size: 512 * 1024 * 1024,
        entries: None,
        metadata_file_size: 1024 * 1024,
        compatibility: TarballCompatibility::Strict,
    };
    const LEGACY: TarballPolicy = TarballPolicy {
        compatibility: TarballCompatibility::Legacy,
        ..STRICT
    };

    fn describe_result(result: Result<TarballInfo, TarballError>) -> String {
        result.map_or_else(|error| error.to_string(), |_| "ok".to_string())
    }

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

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_debug_snapshot!(tarball_info);

        let err = assert_err!(process_tarball("bar-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"invalid path found: foo-0.0.1/Cargo.toml");
    }

    #[tokio::test]
    async fn process_tarball_test_ignores_directories_during_target_discovery() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_dir("foo-0.0.1/build.rs")
            .add_dir("foo-0.0.1/src/lib.rs")
            .add_dir("foo-0.0.1/src/main.rs")
            .add_dir("foo-0.0.1/src/bin/tool.rs")
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);

        assert_none!(tarball_info.manifest.package.unwrap().build);
        assert_none!(tarball_info.manifest.lib);
        assert!(tarball_info.manifest.bin.is_empty());
    }

    #[tokio::test]
    async fn process_tarball_test_unexpected_entry_types() {
        let unexpected_entry_types = [
            tar::EntryType::new(b'Z'),
            tar::EntryType::contiguous(),
            tar::EntryType::new(b'S'),
            tar::EntryType::hard_link(),
            tar::EntryType::symlink(),
            tar::EntryType::character_special(),
            tar::EntryType::block_special(),
            tar::EntryType::fifo(),
        ];

        let mut results = String::new();
        for entry_type in unexpected_entry_types {
            let tarball = tarball_with_entry_type(entry_type);
            let result = process_tarball("foo-0.0.1", &*tarball, STRICT).await;
            let typeflag = entry_type.as_byte() as char;
            results.push_str(&format!("'{typeflag}': {}\n", describe_result(result)));
        }
        assert_snapshot!(results, @"
        'Z': uploaded tarball is invalid: at byte 1024: unsupported tar typeflag 90
        '7': ok
        'S': uploaded tarball is invalid: at byte 1024: unsupported tar typeflag 83
        '1': unexpected tar entry type Link found: foo-0.0.1/bar
        '2': unexpected tar entry type Symlink found: foo-0.0.1/bar
        '3': unexpected tar entry type Char found: foo-0.0.1/bar
        '4': unexpected tar entry type Block found: foo-0.0.1/bar
        '6': unexpected tar entry type Fifo found: foo-0.0.1/bar
        ");
    }

    #[tokio::test]
    async fn process_tarball_test_gnu_long_name() {
        let long_path = format!("foo-0.0.1/{}", "a".repeat(100));
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file(&long_path, b"")
            .build();

        assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
    }

    /// Old versions of the `tar` crate wrote GNU long name payloads without a
    /// NUL terminator. Only legacy mode accepts them.
    #[tokio::test]
    async fn process_tarball_test_gnu_long_name_without_nul() {
        let bin_name = "a".repeat(100);
        let long_path = format!("foo-0.0.1/src/bin/{bin_name}.rs");
        let mut builder = TarballBuilder::new().add_file("foo-0.0.1/Cargo.toml", MANIFEST);

        let mut header = tar::Header::new_gnu();
        header.set_path("././@LongLink").unwrap();
        header.set_entry_type(tar::EntryType::GNULongName);
        header.set_size(long_path.len() as u64);
        header.set_cksum();
        builder
            .as_mut()
            .append(&header, long_path.as_bytes())
            .unwrap();

        // The member header holds the first 100 bytes of the name.
        let mut header = tar::Header::new_gnu();
        header
            .as_old_mut()
            .name
            .copy_from_slice(&long_path.as_bytes()[..100]);
        header.set_size(0);
        header.set_cksum();
        builder.as_mut().append(&header, &[][..]).unwrap();

        let tarball = builder.build();
        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 1024: malformed GNU LongName metadata payload: value is not NUL-terminated");

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, LEGACY).await);

        // Target discovery only finds the binary if the full long name was used.
        let bin = &tarball_info.manifest.bin;
        assert_eq!(bin.len(), 1);
        assert_eq!(bin[0].name.as_deref(), Some(bin_name.as_str()));
    }

    #[tokio::test]
    async fn process_tarball_test_ustar_headers() {
        let tarball = TarballBuilder::new()
            .add_file_with_header(tar::Header::new_ustar(), "foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file_with_header(tar::Header::new_ustar(), "foo-0.0.1/src/lib.rs", b"")
            .build();

        assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
    }

    #[tokio::test]
    async fn process_tarball_test_mixed_header_formats() {
        let tarball = TarballBuilder::new()
            .add_file_with_header(tar::Header::new_ustar(), "foo-0.0.1/src/lib.rs", b"")
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 512: archive format changed from Pax to Gnu");

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, LEGACY).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 512: archive format changed from Pax to Gnu");
    }

    #[tokio::test]
    async fn process_tarball_test_base256_ustar_fields() {
        fn tarball_with_base256_field(field: &str) -> Vec<u8> {
            let mut header = tar::Header::new_ustar();
            header.set_path("foo-0.0.1/src/lib.rs").unwrap();
            header.set_size(0);

            let ustar = header.as_ustar_mut().unwrap();
            let bytes: &mut [u8] = match field {
                "mode" => &mut ustar.mode,
                "uid" => &mut ustar.uid,
                "gid" => &mut ustar.gid,
                "size" => &mut ustar.size,
                "mtime" => &mut ustar.mtime,
                _ => unreachable!(),
            };
            // The GNU base-256 encoding of zero.
            bytes.fill(0);
            bytes[0] = 0x80;
            header.set_cksum();

            let mut builder = TarballBuilder::new();
            builder.as_mut().append(&header, &[][..]).unwrap();
            builder
                .add_file_with_header(tar::Header::new_ustar(), "foo-0.0.1/Cargo.toml", MANIFEST)
                .build()
        }

        let mut results = String::new();
        for field in ["mode", "uid", "gid", "size", "mtime"] {
            let tarball = tarball_with_base256_field(field);

            let result = process_tarball("foo-0.0.1", &*tarball, STRICT).await;
            results.push_str(&format!("{field} (strict): {}\n", describe_result(result)));

            let result = process_tarball("foo-0.0.1", &*tarball, LEGACY).await;
            results.push_str(&format!("{field} (legacy): {}\n", describe_result(result)));
        }
        assert_snapshot!(results, @"
        mode (strict): uploaded tarball is invalid: at byte 0: invalid tar mode field: found [128, 0, 0, 0, 0, 0, 0, 0]
        mode (legacy): uploaded tarball is invalid: at byte 0: invalid tar mode field: found [128, 0, 0, 0, 0, 0, 0, 0]
        uid (strict): uploaded tarball is invalid: at byte 0: invalid tar uid field: found [128, 0, 0, 0, 0, 0, 0, 0]
        uid (legacy): ok
        gid (strict): uploaded tarball is invalid: at byte 0: invalid tar gid field: found [128, 0, 0, 0, 0, 0, 0, 0]
        gid (legacy): ok
        size (strict): uploaded tarball is invalid: at byte 0: invalid tar size field: found [128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        size (legacy): uploaded tarball is invalid: at byte 0: invalid tar size field: found [128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        mtime (strict): uploaded tarball is invalid: at byte 0: invalid tar mtime field: found [128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        mtime (legacy): uploaded tarball is invalid: at byte 0: invalid tar mtime field: found [128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        ");

        // GNU headers support base-256 numbers, which the `tar` crate uses for
        // IDs that do not fit into seven octal digits.
        let mut header = tar::Header::new_gnu();
        header.set_uid(0x4080_2a6e);
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file_with_header(header, "foo-0.0.1/src/lib.rs", b"")
            .build();

        assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
    }

    #[tokio::test]
    async fn process_tarball_test_pax_vendor_extensions() {
        let tarball_with_pax_keyword = |keyword| {
            TarballBuilder::ustar()
                .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
                .add_pax_extensions([(keyword, b"value".as_slice())])
                .add_file("foo-0.0.1/src/lib.rs", b"")
                .build()
        };

        let tarball = tarball_with_pax_keyword("LIBARCHIVE.xattr.com.apple.provenance");
        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 1024: decode policy rejected input: pax vendor extension LIBARCHIVE.xattr.com.apple.provenance is not allowed");
        assert_ok!(process_tarball("foo-0.0.1", &*tarball, LEGACY).await);

        // Standard keywords and the `realtime` and `security` namespaces are
        // not vendor extensions.
        for keyword in ["comment", "realtime.attribute", "security.attribute"] {
            let tarball = tarball_with_pax_keyword(keyword);
            assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        }
    }

    #[tokio::test]
    async fn process_tarball_test_unknown_pax_keyword() {
        let tarball = TarballBuilder::ustar()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_pax_extensions([("foo", b"value".as_slice())])
            .add_file("foo-0.0.1/src/lib.rs", b"")
            .build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @r#"uploaded tarball is invalid: at byte 1024: invalid or unknown pax keyword "foo""#);

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, LEGACY).await);
        assert_snapshot!(err, @r#"uploaded tarball is invalid: at byte 1024: invalid or unknown pax keyword "foo""#);
    }

    #[tokio::test]
    async fn process_tarball_test_global_pax_header() {
        let mut builder = TarballBuilder::new();

        let records = b"17 comment=hello\n";
        let mut header = tar::Header::new_ustar();
        header.set_path("pax_global_header").unwrap();
        header.set_entry_type(tar::EntryType::XGlobalHeader);
        header.set_size(records.len() as u64);
        header.set_cksum();
        builder.as_mut().append(&header, &records[..]).unwrap();

        let tarball = builder
            .add_file_with_header(tar::Header::new_ustar(), "foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 0: decode policy rejected input: global pax extended headers are not allowed");

        assert_ok!(process_tarball("foo-0.0.1", &*tarball, LEGACY).await);
    }

    #[tokio::test]
    async fn process_tarball_test_directory_with_payload() {
        // The directory payload holds the header of a binary target, which
        // readers that honor the directory size skip over.
        let mut hidden = tar::Header::new_gnu();
        hidden.set_path("foo-0.0.1/src/main.rs").unwrap();
        hidden.set_size(0);
        hidden.set_cksum();

        let mut header = tar::Header::new_gnu();
        header.set_path("foo-0.0.1/src/").unwrap();
        header.set_entry_type(tar::EntryType::Directory);
        header.set_size(512);
        header.set_cksum();

        let mut builder = TarballBuilder::new().add_file("foo-0.0.1/Cargo.toml", MANIFEST);
        builder
            .as_mut()
            .append(&header, &hidden.as_bytes()[..])
            .unwrap();
        let tarball = builder.build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 1024: member type Directory cannot carry payload size 512");

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, LEGACY).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 1024: member type Directory cannot carry payload size 512");
    }

    #[tokio::test]
    async fn process_tarball_test_data_after_end_of_archive() {
        let mut archive = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build_unzipped();
        archive.extend([0xff; 512]);

        let mut tarball = vec![];
        flate2::read::GzEncoder::new(&*archive, flate2::Compression::default())
            .read_to_end(&mut tarball)
            .unwrap();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 2048: unexpected data after the end-of-archive marker");

        assert_ok!(process_tarball("foo-0.0.1", &*tarball, LEGACY).await);
    }

    #[tokio::test]
    async fn process_tarball_test_old_headers() {
        let tarball = TarballBuilder::new()
            .add_file_with_header(tar::Header::new_old(), "foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 0: invalid tar identity: found [0, 0, 0, 0, 0, 0, 0, 0]");

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, LEGACY).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 0: invalid tar identity: found [0, 0, 0, 0, 0, 0, 0, 0]");
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
        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
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

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 1024: path is not valid UTF-8");
    }

    #[tokio::test]
    async fn process_tarball_test_size_limit() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let policy = TarballPolicy {
            unpack_size: tarball.len() as u64 - 1,
            ..STRICT
        };
        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, policy).await);
        assert_snapshot!(err, @"uploaded tarball is malformed or too large when decompressed");
    }

    #[tokio::test]
    async fn process_tarball_test_entry_limit() {
        let tarball = TarballBuilder::ustar()
            .add_pax_extensions([("comment", b"metadata".as_slice())])
            .add_dir("foo-0.0.1")
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_dir("foo-0.0.1/src")
            .add_file("foo-0.0.1/src/lib.rs", b"pub fn foo() {}")
            .build();

        let policy = TarballPolicy {
            entries: Some(4),
            ..STRICT
        };
        assert_ok!(process_tarball("foo-0.0.1", &*tarball, policy).await);

        let policy = TarballPolicy {
            entries: Some(3),
            ..STRICT
        };
        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, policy).await);
        assert_snapshot!(err, @"uploaded tarball contains more than 3 entries");
    }

    #[tokio::test]
    async fn process_tarball_test_metadata_size_limit() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let policy = TarballPolicy {
            metadata_file_size: MANIFEST.len() as u64,
            ..STRICT
        };
        assert_ok!(process_tarball("foo-0.0.1", &*tarball, policy).await);

        let policy = TarballPolicy {
            metadata_file_size: MANIFEST.len() as u64 - 1,
            ..STRICT
        };
        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, policy).await);
        assert_snapshot!(err, @"metadata file `foo-0.0.1/Cargo.toml` exceeds the maximum size of 40 bytes");

        let vcs_info = br#"{"path_in_vcs": "a/path/that/is/longer/than/the/manifest"}"#;
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/.cargo_vcs_info.json", vcs_info)
            .build();

        let policy = TarballPolicy {
            metadata_file_size: MANIFEST.len() as u64,
            ..STRICT
        };
        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, policy).await);
        assert_snapshot!(err, @"metadata file `foo-0.0.1/.cargo_vcs_info.json` exceeds the maximum size of 41 bytes");

        // Entries other than the two metadata files are not affected by the limit.
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/src/lib.rs", &[b'a'; 4096])
            .build();

        let policy = TarballPolicy {
            metadata_file_size: MANIFEST.len() as u64,
            ..STRICT
        };
        assert_ok!(process_tarball("foo-0.0.1", &*tarball, policy).await);
    }

    #[tokio::test]
    async fn process_tarball_test_incomplete_vcs_info() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/.cargo_vcs_info.json", br#"{"unknown": "field"}"#)
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
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

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
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

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
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

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_debug_snapshot!(tarball_info);
    }

    #[tokio::test]
    async fn process_tarball_test_manifest_with_default_readme() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
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

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
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

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_debug_snapshot!(tarball_info);
    }

    #[tokio::test]
    async fn process_tarball_test_incorrect_manifest_casing() {
        let process = async |file| {
            let tarball = TarballBuilder::new()
                .add_file(&format!("foo-0.0.1/{file}"), MANIFEST)
                .build();

            process_tarball("foo-0.0.1", &*tarball, STRICT).await
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

            process_tarball("foo-0.0.1", &*tarball, STRICT).await
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

        let tarball = TarballBuilder::ustar()
            .add_pax_extensions([("size", b"\xff\xfe".as_slice())])
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 0: pax records contain invalid UTF-8 text");

        let tarball = TarballBuilder::ustar()
            .add_pax_extensions([("size", b"not-a-valid-number".as_slice())])
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @r#"uploaded tarball is invalid: at byte 0: invalid pax size value: "not-a-valid-number""#);
    }

    #[tokio::test]
    async fn process_tarball_test_size_mismatch() {
        // Build up a tarball with a mismatch between its pax and tar header
        // sizes.
        let tarball = TarballBuilder::ustar()
            // Set the pax size to a larger value.
            .add_pax_extensions([("size", "2048".as_bytes())])
            // Add the real content, which is less than a single tar block (512
            // bytes).
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            // Add a symlink in the overlap.
            .add_symlink("smuggled", "/etc/issue")
            .build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 1024: pax size 2048 does not match header size 41");
    }

    #[tokio::test]
    async fn process_tarball_test_size_mismatch_with_larger_header_size() {
        let manifest_size = MANIFEST.len().to_string();
        let mut builder =
            TarballBuilder::ustar().add_pax_extensions([("size", manifest_size.as_bytes())]);

        let mut header = tar::Header::new_ustar();
        header.set_path("foo-0.0.1/Cargo.toml").unwrap();
        header.set_size(1024);
        header.set_cksum();
        builder.as_mut().append(&header, MANIFEST).unwrap();
        let tarball = builder.build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 1024: pax size 41 does not match header size 1024");
    }

    /// The reader skips the unread payload of the first file in more than one
    /// chunk before it reads the header of the manifest.
    #[tokio::test]
    async fn process_tarball_test_pax_size_after_large_payload() {
        let payload = vec![b'a'; 1024 * 1024 + 7];
        let payload_size = payload.len().to_string();
        let manifest_size = MANIFEST.len().to_string();
        let tarball = TarballBuilder::ustar()
            .add_pax_extensions([("size", payload_size.as_bytes())])
            .add_file("foo-0.0.1/src/lib.rs", &payload)
            .add_pax_extensions([("size", manifest_size.as_bytes())])
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
    }

    #[tokio::test]
    async fn process_tarball_test_matching_pax_size() {
        let manifest_size = MANIFEST.len().to_string();
        let tarball = TarballBuilder::ustar()
            .add_pax_extensions([("size", manifest_size.as_bytes())])
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
    }

    #[tokio::test]
    async fn process_tarball_test_repeated_pax_size_mismatch() {
        let manifest_size = MANIFEST.len().to_string();
        // The final size matches the tar header, but every size record must match.
        let tarball = TarballBuilder::ustar()
            .add_pax_extensions([
                ("size", "2048".as_bytes()),
                ("size", manifest_size.as_bytes()),
            ])
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_symlink("smuggled", "/etc/issue")
            .build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 0: decode policy rejected input: pax extended header contains duplicate record size");
    }

    #[tokio::test]
    async fn process_tarball_test_repeated_pax_path() {
        // Pax gives the last `path` record precedence, but the `tar` crate
        // that Cargo uses for extraction picks the first one.
        let tarball = TarballBuilder::ustar()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_pax_extensions([
                ("path", "foo-0.0.1/build.rs".as_bytes()),
                ("path", "foo-0.0.1/src/lib.rs".as_bytes()),
            ])
            .add_file("foo-0.0.1/src/lib.rs", b"fn main() {}")
            .build();

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 1024: decode policy rejected input: pax extended header contains duplicate record path");
    }

    #[tokio::test]
    async fn process_tarball_test_multiple_gzip_members() {
        // A tar archive ends with two 512-byte blocks of zeros.
        const END_OF_ARCHIVE_LEN: usize = 2 * 512;

        // Without its end-of-archive blocks, the tar stream continues in the
        // second gzip member, which only multi-member gzip decoders like
        // `gzip` and `bsdtar` read.
        let mut first = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/src/lib.rs", b"")
            .build_unzipped();
        first.truncate(first.len() - END_OF_ARCHIVE_LEN);

        let mut tarball = vec![];
        flate2::read::GzEncoder::new(&*first, flate2::Compression::default())
            .read_to_end(&mut tarball)
            .unwrap();

        let second = TarballBuilder::new()
            .add_file("foo-0.0.1/build.rs", b"fn main() {}")
            .build();
        tarball.extend(second);

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 1536: missing two-block end-of-archive marker");

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, LEGACY).await);
        assert_snapshot!(err, @"uploaded tarball is invalid: at byte 1536: missing two-block end-of-archive marker");
    }

    #[tokio::test]
    async fn process_tarball_test_trailing_data() {
        let mut tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();
        tarball.push(0);

        let err = assert_err!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_snapshot!(err, @"uploaded tarball contains data after the end of the gzip stream");

        assert_ok!(process_tarball("foo-0.0.1", &*tarball, LEGACY).await);
    }

    #[tokio::test]
    async fn test_lib() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/src/lib.rs", b"pub fn foo() {}")
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
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

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_debug_snapshot!(tarball_info);
    }

    #[tokio::test]
    async fn test_app() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/src/main.rs", b"fn main() {}")
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, STRICT).await);
        assert_debug_snapshot!(tarball_info);
    }
}
