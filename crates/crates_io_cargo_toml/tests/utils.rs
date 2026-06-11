use tempfile::TempDir;

pub fn prepare(manifest: &str, extra_files: Vec<&str>) -> TempDir {
    let tempdir = tempfile::tempdir().unwrap();

    // Create `Cargo.toml` manifest file
    std::fs::write(tempdir.path().join("Cargo.toml"), manifest).unwrap();

    // Create extra files
    for file in extra_files {
        let path = tempdir.path().join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }

    tempdir
}
