use claims::{assert_err, assert_ok, assert_ok_eq};
use crates_io_crate_zip::{FileEntry, Manifest, build_zip};
use crates_io_tarball::TarballBuilder;
use insta::assert_snapshot;
use sha2::{Digest, Sha256};
use std::io::Cursor;

const CARGO_TOML: &[u8] = b"[package]\nname = \"crate\"\nversion = \"1.0.0\"\n";

fn build() -> (Vec<u8>, Manifest) {
    let input = TarballBuilder::new()
        .add_file("crate-1.0.0/Cargo.toml", CARGO_TOML)
        .build();
    let mut input = Cursor::new(input);
    let mut output = Cursor::new(Vec::new());
    let modified = assert_ok!(zip::DateTime::from_date_and_time(2020, 1, 2, 3, 4, 6));
    let manifest = assert_ok!(build_zip(&mut input, modified, &mut output));

    (output.into_inner(), manifest)
}

fn cargo_toml_payload<'a>(zip: &'a [u8], manifest: &Manifest) -> (&'a [u8], FileEntry) {
    let entry = assert_ok!(manifest.cargo_toml()).clone();
    let range = assert_ok!(entry.data_range());
    let payload = &zip[range.start as usize..range.end as usize];

    (payload, entry)
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn stored_entry(contents: &[u8]) -> FileEntry {
    FileEntry {
        path: "Cargo.toml".into(),
        data_offset: 0,
        compressed_size: contents.len() as u64,
        uncompressed_size: contents.len() as u64,
        compression: "store".into(),
        sha256: sha256_hex(contents),
    }
}

#[test]
fn decodes_deflated_entries_from_their_data_range() {
    let (zip, manifest) = build();
    let (payload, entry) = cargo_toml_payload(&zip, &manifest);

    assert_ok_eq!(entry.decode(payload), CARGO_TOML);
}

#[test]
fn rejects_truncated_compressed_data() {
    let (zip, manifest) = build();
    let (payload, entry) = cargo_toml_payload(&zip, &manifest);

    let error = assert_err!(entry.decode(&payload[..payload.len() - 1]));
    assert_snapshot!(error, @"Expected 43 compressed bytes for `Cargo.toml`, got 42");
}

#[test]
fn rejects_invalid_uncompressed_size() {
    let (zip, manifest) = build();
    let (payload, entry) = cargo_toml_payload(&zip, &manifest);

    let mut entry = entry;
    entry.uncompressed_size -= 1;
    let error = assert_err!(entry.decode(payload));
    assert_snapshot!(error, @"Expected 42 uncompressed bytes for `Cargo.toml`, got at least 43");
}

#[test]
fn rejects_invalid_hash() {
    let (zip, manifest) = build();
    let (payload, mut entry) = cargo_toml_payload(&zip, &manifest);

    entry.sha256 = "00".repeat(32);
    let error = assert_err!(entry.decode(payload));
    assert_snapshot!(error, @"SHA-256 mismatch for `Cargo.toml`");
}

#[test]
fn rejects_data_range_overflow() {
    let mut entry = stored_entry(b"x");
    entry.data_offset = u64::MAX;

    let error = assert_err!(entry.data_range());
    assert_snapshot!(error, @"Data range overflow for `Cargo.toml`");
}

#[test]
fn rejects_unknown_compression() {
    let mut entry = stored_entry(b"");
    entry.compression = "unknown".into();

    let error = assert_err!(entry.decode(b""));
    assert_snapshot!(error, @"Unsupported compression `unknown` for `Cargo.toml`");
}

#[test]
fn rejects_uncompressed_size_overflow() {
    let mut entry = stored_entry(b"");
    entry.uncompressed_size = u64::MAX;

    let error = assert_err!(entry.decode(b""));
    assert_snapshot!(error, @"Uncompressed size overflow for `Cargo.toml`");
}

#[test]
fn decodes_stored_entries() {
    let contents = b"[package]\n";
    let entry = stored_entry(contents);

    assert_ok_eq!(entry.decode(contents), contents);
}
