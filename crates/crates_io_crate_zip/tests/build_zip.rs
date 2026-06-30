use claims::{assert_err, assert_none, assert_ok, assert_some, assert_some_eq};
use crates_io_crate_zip::{FileEntry, Manifest, build_zip};
use crates_io_tarball::TarballBuilder;
use flate2::read::DeflateDecoder;
use insta::assert_snapshot;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};

/// A small `.crate` fixture: a gz tarball wrapping a `crate-1.0.0/` directory
/// with a `Cargo.toml`, two source files, an empty file, `build.rs`,
/// `README.md`, and an explicit directory entry (to exercise the non-file
/// filter). The entries are added out of sorted order, `build.rs` sorts before
/// `Cargo.toml`, and `README.md` is capitalized, so the fixture exercises both
/// the case-insensitive sort and the difference between manifest order and the
/// physical `Cargo.toml`-first zip layout.
fn test_crate() -> Vec<u8> {
    TarballBuilder::new()
        .add_file("crate-1.0.0/src/lib.rs", b"pub fn foo() {}\n")
        .add_file("crate-1.0.0/src/main.rs", b"fn main() {}\n")
        .add_file(
            "crate-1.0.0/Cargo.toml",
            b"[package]\nname = \"crate\"\nversion = \"1.0.0\"\n",
        )
        .add_file("crate-1.0.0/empty.txt", b"")
        .add_file("crate-1.0.0/README.md", b"# crate\n")
        .add_file("crate-1.0.0/build.rs", b"fn main() {}\n")
        .add_dir("crate-1.0.0/src/")
        .build()
}

fn modified() -> zip::DateTime {
    assert_ok!(zip::DateTime::from_date_and_time(2020, 1, 2, 3, 4, 6))
}

fn build(input: &[u8]) -> (Vec<u8>, Manifest) {
    let mut reader = Cursor::new(input.to_vec());
    let mut out = Cursor::new(Vec::new());
    let manifest = assert_ok!(build_zip(&mut reader, modified(), &mut out));
    (out.into_inner(), manifest)
}

/// Reads one manifest entry's payload straight out of the raw zip bytes using
/// only `data_offset` / `compressed_size` / `compression`, mirroring what a
/// range-fetching consumer would do.
fn payload_from_offsets(zip_bytes: &[u8], file: &FileEntry) -> Vec<u8> {
    let start = file.data_offset as usize;
    let end = start + file.compressed_size as usize;
    let payload = &zip_bytes[start..end];
    match file.compression.as_str() {
        "deflate" => {
            let mut out = Vec::new();
            assert_ok!(DeflateDecoder::new(payload).read_to_end(&mut out));
            out
        }
        "store" => payload.to_vec(),
        other => panic!("unexpected compression: {other}"),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[test]
fn happy_path() {
    let (_, manifest) = build(&test_crate());

    // The path list, inline for an at-a-glance check that the `crate-1.0.0/`
    // prefix is stripped, the directory entry is filtered out, and the files
    // are sorted case-insensitively (`build.rs` before `Cargo.toml`).
    let paths = manifest.files.iter().map(|f| f.path.as_str());
    let paths = paths.collect::<Vec<_>>();
    insta::assert_debug_snapshot!(paths, @r#"
    [
        "build.rs",
        "Cargo.toml",
        "empty.txt",
        "README.md",
        "src/lib.rs",
        "src/main.rs",
    ]
    "#);

    // The full manifest, including per-file compression, sizes, offsets, and
    // sha256.
    insta::assert_debug_snapshot!(manifest);
}

#[test]
fn determinism() {
    let input = test_crate();
    let (zip_a, manifest_a) = build(&input);
    let (zip_b, manifest_b) = build(&input);
    assert_eq!(zip_a, zip_b);
    assert_eq!(manifest_a, manifest_b);
}

#[test]
fn entries_use_the_given_timestamp() {
    let expected = modified();

    // Guards the determinism mechanism: every entry must carry the explicit
    // `modified` time, not the current wall-clock time.
    let (zip_bytes, _) = build(&test_crate());
    let mut archive = assert_ok!(zip::ZipArchive::new(Cursor::new(zip_bytes)));
    for i in 0..archive.len() {
        let entry = assert_ok!(archive.by_index(i));
        assert_some_eq!(entry.last_modified(), expected);
    }
}

#[test]
fn case_only_duplicate_paths_sort_deterministically() {
    let input = TarballBuilder::new()
        .add_file("crate-1.0.0/Cargo.toml", b"[package]\n")
        .add_file("crate-1.0.0/Config.toml", b"upper\n")
        .add_file("crate-1.0.0/config.toml", b"lower\n")
        .build();

    let (_, first) = build(&input);
    let (_, second) = build(&input);
    assert_eq!(first, second);

    let paths = first.files.iter().map(|f| f.path.as_str());
    let paths = paths.collect::<Vec<_>>();
    insta::assert_debug_snapshot!(paths, @r#"
    [
        "Cargo.toml",
        "Config.toml",
        "config.toml",
    ]
    "#);
}

#[test]
fn duplicate_paths_keep_the_last_occurrence() {
    // A tarball can carry several entries with the same path: an exact-duplicate
    // `Cargo.toml`, or e.g. old crates with two `Cargo.toml.orig` files. The zip
    // cannot hold duplicate names, so we keep the last occurrence, matching
    // cargo's overwrite-on-extract behavior.
    let input = TarballBuilder::new()
        .add_file("crate-1.0.0/Cargo.toml", b"first cargo\n")
        .add_file("crate-1.0.0/Cargo.toml", b"last cargo\n")
        .add_file("crate-1.0.0/dup.txt", b"first dup\n")
        .add_file("crate-1.0.0/dup.txt", b"last dup\n")
        .build();

    let (zip_bytes, manifest) = build(&input);

    for (path, expected) in [
        ("Cargo.toml", b"last cargo\n".as_slice()),
        ("dup.txt", b"last dup\n".as_slice()),
    ] {
        let mut entries = manifest.files.iter().filter(|f| f.path == path);
        let entry = assert_some!(entries.next());
        assert_none!(entries.next());

        // The recorded hash and the bytes actually stored in the zip are both
        // the last occurrence's.
        assert_eq!(entry.sha256, sha256_hex(expected));
        assert_eq!(payload_from_offsets(&zip_bytes, entry).as_slice(), expected);
    }
}

#[test]
fn entry_outside_package_directory_is_an_error() {
    // Every entry in a published `.crate` lives under `{name}-{version}/`. A
    // top-level entry means a malformed tarball, which must be rejected.
    let input = TarballBuilder::new()
        .add_file("rogue.txt", b"oops\n")
        .build();

    let mut reader = Cursor::new(input);
    let mut out = Cursor::new(Vec::new());
    let err = assert_err!(build_zip(&mut reader, modified(), &mut out));
    assert_snapshot!(err, @"Tarball entry `rogue.txt` is not inside the package directory");
}

#[test]
fn cargo_toml_is_physically_first() {
    // `Cargo.toml` is written first in the zip even though it is not first in
    // the sorted manifest, so it has the smallest payload offset.
    let (_, manifest) = build(&test_crate());
    let cargo_toml = assert_some!(manifest.files.iter().find(|f| f.path == "Cargo.toml"));
    let min_offset = assert_some!(manifest.files.iter().map(|f| f.data_offset).min());
    assert_eq!(cargo_toml.data_offset, min_offset);

    // Lowercase `cargo.toml` is also supported for backwards compatibility too.
    let input = TarballBuilder::new()
        .add_file("crate-1.0.0/src/lib.rs", b"pub fn foo() {}\n")
        .add_file("crate-1.0.0/cargo.toml", b"[package]\n")
        .build();

    let (_, manifest) = build(&input);
    let cargo_toml = assert_some!(manifest.files.iter().find(|f| f.path == "cargo.toml"));
    let min_offset = assert_some!(manifest.files.iter().map(|f| f.data_offset).min());
    assert_eq!(cargo_toml.data_offset, min_offset);
}

#[test]
fn offsets_and_hashes_are_real() {
    let (zip_bytes, manifest) = build(&test_crate());

    let mut archive = assert_ok!(zip::ZipArchive::new(Cursor::new(zip_bytes.clone())));

    for file in &manifest.files {
        // Reconstruct the contents purely from the recorded offsets.
        let from_offsets = payload_from_offsets(&zip_bytes, file);
        assert_eq!(file.uncompressed_size, from_offsets.len() as u64);
        assert_eq!(file.sha256, sha256_hex(&from_offsets));

        // Cross-check against the zip crate reading the same entry by name.
        let mut entry = assert_ok!(archive.by_name(&file.path));
        let mut expected = Vec::new();
        assert_ok!(entry.read_to_end(&mut expected));
        assert_eq!(from_offsets, expected);
    }
}
