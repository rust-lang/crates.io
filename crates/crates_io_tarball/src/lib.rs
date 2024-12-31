#![doc = include_str!("../README.md")]

#[cfg(test)]
#[macro_use]
extern crate claims;

#[cfg(any(feature = "builder", test))]
pub use crate::builder::TarballBuilder;
use crate::limit_reader::LimitErrorReader;
use crate::manifest::validate_manifest;
pub use crate::vcs_info::CargoVcsInfo;
use cargo_manifest::AbstractFilesystem;
pub use cargo_manifest::{Manifest, StringOrBool};
use futures_util::StreamExt;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use tokio::io::{AsyncReadExt, BufReader};
use tracing::instrument;

#[cfg(any(feature = "builder", test))]
mod builder;
mod limit_reader;
mod manifest;
mod vcs_info;

const DEFAULT_BUF_SIZE: usize = 128 * 1024;

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
    #[error("unexpected symlink or hard link found: {0}")]
    UnexpectedSymlink(String),
    #[error("Cargo.toml manifest is missing")]
    MissingManifest,
    #[error("Cargo.toml manifest is invalid: {0}")]
    InvalidManifest(#[from] cargo_manifest::Error),
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
    max_unpack: u64,
) -> Result<TarballInfo, TarballError> {
    let tarball = BufReader::with_capacity(DEFAULT_BUF_SIZE, tarball);
    // All our data is currently encoded with gzip
    let decoder = async_compression::tokio::bufread::GzipDecoder::new(tarball);

    // Don't let gzip decompression go into the weeeds, apply a fixed cap after
    // which point we say the decompressed source is "too large".
    let decoder = LimitErrorReader::new(decoder, max_unpack);

    // Use this I/O object now to take a peek inside
    let mut archive = tokio_tar::Archive::new(decoder);

    let pkg_root = Path::new(&pkg_name);

    let mut vcs_info = None;
    let mut paths = Vec::new();
    let mut manifests = BTreeMap::new();
    let mut entries = archive.entries()?;

    while let Some(entry) = entries.next().await {
        let mut entry = entry.map_err(TarballError::Malformed)?;

        // Verify that all entries actually start with `$name-$vers/`.
        // Historically Cargo didn't verify this on extraction so you could
        // upload a tarball that contains both `foo-0.1.0/` source code as well
        // as `bar-0.1.0/` source code, and this could overwrite other crates in
        // the registry!
        let entry_path = entry.path()?;
        let Ok(in_pkg_path) = entry_path.strip_prefix(pkg_root) else {
            return Err(TarballError::InvalidPath(entry_path.display().to_string()));
        };

        // Historical versions of the `tar` crate which Cargo uses internally
        // don't properly prevent hard links and symlinks from overwriting
        // arbitrary files on the filesystem. As a bit of a hammer we reject any
        // tarball with these sorts of links. Cargo doesn't currently ever
        // generate a tarball with these file types so this should work for now.
        let entry_type = entry.header().entry_type();
        if entry_type.is_hard_link() || entry_type.is_symlink() {
            return Err(TarballError::UnexpectedSymlink(
                entry_path.display().to_string(),
            ));
        }

        paths.push(in_pkg_path.to_path_buf());

        // Let's go hunting for the VCS info and crate manifest. The only valid place for these is
        // in the package root in the tarball.
        if entry_path.parent() == Some(pkg_root) {
            let entry_file = entry_path.file_name().unwrap_or_default();
            if entry_file == ".cargo_vcs_info.json" {
                let mut contents = String::new();
                entry.read_to_string(&mut contents).await?;
                vcs_info = CargoVcsInfo::from_contents(&contents).ok();
            } else if entry_file.to_ascii_lowercase() == "cargo.toml" {
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
                // We can skip non-utf8 paths, since those are not checked by `cargo_manifest` anyway
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
    use super::process_tarball;
    use crate::TarballBuilder;
    use cargo_manifest::{MaybeInherited, StringOrBool};
    use insta::{assert_debug_snapshot, assert_snapshot};

    const MANIFEST: &[u8] = b"[package]\nname = \"foo\"\nversion = \"0.0.1\"\n";
    const MAX_SIZE: u64 = 512 * 1024 * 1024;

    #[tokio::test]
    async fn process_tarball_test() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, MAX_SIZE).await);
        assert_none!(tarball_info.vcs_info);
        assert_none!(tarball_info.manifest.lib);
        assert_eq!(tarball_info.manifest.bin, vec![]);
        assert_eq!(tarball_info.manifest.example, vec![]);

        let err = assert_err!(process_tarball("bar-0.0.1", &*tarball, MAX_SIZE).await);
        assert_snapshot!(err, @"invalid path found: foo-0.0.1/Cargo.toml");
    }

    #[tokio::test]
    async fn process_tarball_test_size_limit() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let err =
            assert_err!(process_tarball("foo-0.0.1", &*tarball, tarball.len() as u64 - 1).await);
        assert_snapshot!(err, @"uploaded tarball is malformed or too large when decompressed");
    }

    #[tokio::test]
    async fn process_tarball_test_incomplete_vcs_info() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/.cargo_vcs_info.json", br#"{"unknown": "field"}"#)
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, MAX_SIZE).await);
        let vcs_info = assert_some!(tarball_info.vcs_info);
        assert_eq!(vcs_info.path_in_vcs, "");
    }

    #[tokio::test]
    async fn process_tarball_test_vcs_info() {
        let vcs_info = br#"{"path_in_vcs": "path/in/vcs"}"#;
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/.cargo_vcs_info.json", vcs_info)
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, MAX_SIZE).await);
        let vcs_info = assert_some!(tarball_info.vcs_info);
        assert_eq!(vcs_info.path_in_vcs, "path/in/vcs");
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

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, MAX_SIZE).await);
        let package = assert_some!(tarball_info.manifest.package);
        assert_matches!(package.readme, Some(MaybeInherited::Local(StringOrBool::String(s))) if s == "README.md");
        assert_matches!(package.repository, Some(MaybeInherited::Local(s)) if s ==  "https://github.com/foo/bar");
        assert_matches!(package.rust_version, Some(MaybeInherited::Local(s)) if s == "1.59");
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

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, MAX_SIZE).await);
        let package = assert_some!(tarball_info.manifest.package);
        assert_matches!(package.rust_version, Some(MaybeInherited::Local(s)) if s == "1.23");
    }

    #[tokio::test]
    async fn process_tarball_test_manifest_with_default_readme() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, MAX_SIZE).await);
        let package = assert_some!(tarball_info.manifest.package);
        assert_none!(package.readme);
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

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, MAX_SIZE).await);
        let package = assert_some!(tarball_info.manifest.package);
        assert_matches!(package.readme, Some(MaybeInherited::Local(StringOrBool::Bool(b))) if !b);
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

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, MAX_SIZE).await);
        let package = assert_some!(tarball_info.manifest.package);
        assert_matches!(package.repository, Some(MaybeInherited::Local(s)) if s ==  "https://github.com/foo/bar");
    }

    #[tokio::test]
    async fn process_tarball_test_incorrect_manifest_casing() {
        let process = |file| async move {
            let tarball = TarballBuilder::new()
                .add_file(&format!("foo-0.0.1/{file}"), MANIFEST)
                .build();

            process_tarball("foo-0.0.1", &*tarball, MAX_SIZE).await
        };

        let err = assert_err!(process("CARGO.TOML").await);
        assert_snapshot!(err, @r#"Cargo.toml manifest is incorrectly cased: "CARGO.TOML""#);

        let err = assert_err!(process("Cargo.Toml").await);
        assert_snapshot!(err, @r#"Cargo.toml manifest is incorrectly cased: "Cargo.Toml""#);
    }

    #[tokio::test]
    async fn process_tarball_test_multiple_manifests() {
        let process = |files: Vec<_>| async move {
            let tarball = files
                .iter()
                .fold(TarballBuilder::new(), |builder, file| {
                    builder.add_file(&format!("foo-0.0.1/{file}"), MANIFEST)
                })
                .build();

            process_tarball("foo-0.0.1", &*tarball, MAX_SIZE).await
        };

        let err = assert_err!(process(vec!["cargo.toml", "Cargo.toml"]).await);
        assert_snapshot!(err, @r#"more than one Cargo.toml manifest in tarball: ["foo-0.0.1/Cargo.toml", "foo-0.0.1/cargo.toml"]"#);

        let err = assert_err!(process(vec!["Cargo.toml", "Cargo.Toml"]).await);
        assert_snapshot!(err, @r#"more than one Cargo.toml manifest in tarball: ["foo-0.0.1/Cargo.Toml", "foo-0.0.1/Cargo.toml"]"#);

        let err = assert_err!(process(vec!["Cargo.toml", "cargo.toml", "CARGO.TOML"]).await);
        assert_snapshot!(err, @r#"more than one Cargo.toml manifest in tarball: ["foo-0.0.1/CARGO.TOML", "foo-0.0.1/Cargo.toml", "foo-0.0.1/cargo.toml"]"#);
    }

    #[tokio::test]
    async fn test_lib() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/src/lib.rs", b"pub fn foo() {}")
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, MAX_SIZE).await);
        let lib = assert_some!(tarball_info.manifest.lib);
        assert_debug_snapshot!(lib);
        assert_eq!(tarball_info.manifest.bin, vec![]);
        assert_eq!(tarball_info.manifest.example, vec![]);
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

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, MAX_SIZE).await);
        let lib = assert_some!(tarball_info.manifest.lib);
        assert_debug_snapshot!(lib);
        assert_debug_snapshot!(tarball_info.manifest.bin);
        assert_debug_snapshot!(tarball_info.manifest.example);
    }

    #[tokio::test]
    async fn test_app() {
        let tarball = TarballBuilder::new()
            .add_file("foo-0.0.1/Cargo.toml", MANIFEST)
            .add_file("foo-0.0.1/src/main.rs", b"fn main() {}")
            .build();

        let tarball_info = assert_ok!(process_tarball("foo-0.0.1", &*tarball, MAX_SIZE).await);
        assert_none!(tarball_info.manifest.lib);
        assert_debug_snapshot!(tarball_info.manifest.bin);
        assert_eq!(tarball_info.manifest.example, vec![]);
    }
}
