use std::{
    fs::File,
    path::{Path, PathBuf},
};

use crate::{background_jobs::Environment, uploaders::Uploader, util::errors::std_error_no_send};

use swirl::PerformError;

/// Create CSV dumps of the public information in the database, wrap them in a
/// tarball and upload to S3.
#[swirl::background_job]
pub fn dump_db(
    env: &Environment,
    database_url: String,
    target_name: String,
) -> Result<(), PerformError> {
    let directory = DumpDirectory::create()?;
    directory.dump_db(&database_url)?;
    let tarball = DumpTarball::create(&directory.export_dir)?;
    tarball.upload(&target_name, &env.uploader)?;
    println!("Database dump uploaded to {}.", &target_name);
    Ok(())
}

/// Manage the export directory.
///
/// Create the directory, populate it with the psql scripts and CSV dumps, and
/// make sure it gets deleted again even in the case of an error.
#[derive(Debug)]
pub struct DumpDirectory {
    pub export_dir: PathBuf,
}

impl DumpDirectory {
    pub fn create() -> Result<Self, PerformError> {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
        let export_dir = std::env::temp_dir().join("dump-db").join(timestamp);
        std::fs::create_dir_all(&export_dir)?;
        Ok(Self { export_dir })
    }

    pub fn dump_db(&self, database_url: &str) -> Result<(), PerformError> {
        let export_script = self.export_dir.join("export.sql");
        let import_script = self.export_dir.join("import.sql");
        gen_scripts::gen_scripts(&export_script, &import_script)?;
        std::fs::create_dir(self.export_dir.join("data"))?;
        run_psql(&export_script, database_url)
    }
}

impl Drop for DumpDirectory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.export_dir).unwrap();
    }
}

pub fn run_psql(script: &Path, database_url: &str) -> Result<(), PerformError> {
    let psql_script = File::open(&script)?;
    let psql = std::process::Command::new("psql")
        .arg(database_url)
        .current_dir(script.parent().unwrap())
        .stdin(psql_script)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    let output = psql.wait_with_output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("ERROR") {
        Err(format!("Error while executing psql: {}", stderr))?;
    }
    if !output.status.success() {
        Err("psql did not finish successfully.")?;
    }
    Ok(())
}

/// Manage the tarball of the database dump.
///
/// Create the tarball, upload it to S3, and make sure it gets deleted.
struct DumpTarball {
    tarball_path: PathBuf,
}

impl DumpTarball {
    fn create(export_dir: &Path) -> Result<Self, PerformError> {
        let tarball_path = export_dir.with_extension("tar.gz");
        let tarfile = File::create(&tarball_path)?;
        let result = Self { tarball_path };
        let encoder = flate2::write::GzEncoder::new(tarfile, flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        archive.append_dir_all(export_dir.file_name().unwrap(), &export_dir)?;
        Ok(result)
    }

    fn upload(&self, target_name: &str, uploader: &Uploader) -> Result<(), PerformError> {
        let client = reqwest::Client::new();
        let tarfile = File::open(&self.tarball_path)?;
        let content_length = tarfile.metadata()?.len();
        // TODO Figure out the correct content type.
        uploader
            .upload(
                &client,
                target_name,
                tarfile,
                content_length,
                "application/gzip",
            )
            .map_err(std_error_no_send)?;
        Ok(())
    }
}

impl Drop for DumpTarball {
    fn drop(&mut self) {
        std::fs::remove_file(&self.tarball_path).unwrap();
    }
}

mod gen_scripts;
