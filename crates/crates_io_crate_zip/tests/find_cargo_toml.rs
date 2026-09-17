use claims::{assert_err, assert_ok_eq};
use crates_io_crate_zip::{FileEntry, Manifest};
use insta::assert_snapshot;

fn file(path: &str) -> FileEntry {
    FileEntry {
        path: path.into(),
        data_offset: 0,
        compressed_size: 0,
        uncompressed_size: 0,
        compression: "store".into(),
        sha256: String::new(),
    }
}

#[test]
fn finds_cargo_toml_case_insensitively() {
    for path in ["Cargo.toml", "cargo.toml"] {
        let manifest = Manifest {
            files: vec![file("src/lib.rs"), file(path)],
        };

        assert_ok_eq!(manifest.cargo_toml().map(|file| file.path.as_str()), path);
    }
}

#[test]
fn rejects_missing_cargo_toml() {
    let manifest = Manifest { files: Vec::new() };
    let error = assert_err!(manifest.cargo_toml());
    assert_snapshot!(error, @"ZIP manifest contains no `Cargo.toml` entry");
}
