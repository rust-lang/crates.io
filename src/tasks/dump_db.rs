use std::{
    fs::File,
    path::{Path, PathBuf},
};

use crate::{background_jobs::Environment, uploaders::Uploader, util::errors::std_error_no_send};

use scopeguard::defer;
use swirl::PerformError;

/// Create CSV dumps of the public information in the database, wrap them in a
/// tarball and upload to S3.
#[swirl::background_job]
pub fn dump_db(
    env: &Environment,
    database_url: String,
    target_name: String,
) -> Result<(), PerformError> {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
    let export_dir = std::env::temp_dir().join("dump-db").join(timestamp);
    std::fs::create_dir_all(&export_dir)?;
    defer! {{
        std::fs::remove_dir_all(&export_dir).unwrap();
    }}
    let export_script = export_dir.join("export.sql");
    let import_script = export_dir.join("import.sql");
    gen_scripts::gen_scripts(&export_script, &import_script)?;
    run_psql(&database_url, &export_script)?;
    let tarball = create_tarball(&export_dir)?;
    defer! {{
        std::fs::remove_file(&tarball).unwrap();
    }}
    upload_tarball(&tarball, &target_name, &env.uploader)?;
    println!("Database dump uploaded to {}.", &target_name);
    Ok(())
}

fn run_psql(database_url: &str, export_script: &Path) -> Result<(), PerformError> {
    use std::process::{Command, Stdio};

    let psql_script = File::open(export_script)?;
    let psql = Command::new("psql")
        .arg(database_url)
        .current_dir(export_script.parent().unwrap())
        .stdin(psql_script)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let output = psql.wait_with_output()?;
    if !output.stderr.is_empty() {
        Err(format!(
            "Error while executing psql: {}",
            String::from_utf8_lossy(&output.stderr)
        ))?;
    }
    if !output.status.success() {
        Err("psql did not finish successfully.")?;
    }
    Ok(())
}

fn create_tarball(export_dir: &Path) -> Result<PathBuf, PerformError> {
    let tarball_name = export_dir.with_extension("tar.gz");
    let tarball = File::create(&tarball_name)?;
    let encoder = flate2::write::GzEncoder::new(tarball, flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);
    archive.append_dir_all(export_dir.file_name().unwrap(), &export_dir)?;
    Ok(tarball_name)
}

fn upload_tarball(
    tarball: &Path,
    target_name: &str,
    uploader: &Uploader,
) -> Result<(), PerformError> {
    let client = reqwest::Client::new();
    let tarfile = File::open(tarball)?;
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

mod gen_scripts;
