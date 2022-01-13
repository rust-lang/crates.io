use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use self::configuration::VisibilityConfig;
use crate::{background_jobs::Environment, uploaders::Uploader};
use reqwest::header;
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

    println!("Begin exporting database");
    directory.populate(&database_url)?;

    println!("Creating tarball");
    let tarball = DumpTarball::create(&directory.export_dir)?;

    println!("Uploading tarball");
    let size = tarball.upload(&target_name, &env.uploader)?;
    println!("Database dump uploaded {} bytes to {}.", size, &target_name);
    Ok(())
}

/// Manage the export directory.
///
/// Create the directory, populate it with the psql scripts and CSV dumps, and
/// make sure it gets deleted again even in the case of an error.
#[derive(Debug)]
pub struct DumpDirectory {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub export_dir: PathBuf,
}

impl DumpDirectory {
    pub fn create() -> Result<Self, PerformError> {
        let timestamp = chrono::Utc::now();
        let timestamp_str = timestamp.format("%Y-%m-%d-%H%M%S").to_string();
        let export_dir = std::env::temp_dir().join("dump-db").join(timestamp_str);
        debug!(?export_dir, "Creating database dump folder…");
        std::fs::create_dir_all(&export_dir)?;
        Ok(Self {
            timestamp,
            export_dir,
        })
    }

    pub fn populate(&self, database_url: &str) -> Result<(), PerformError> {
        self.add_readme()?;
        self.add_metadata()?;
        self.dump_schema(database_url)?;
        self.dump_db(database_url)
    }

    fn add_readme(&self) -> Result<(), PerformError> {
        use std::io::Write;

        let path = self.export_dir.join("README.md");
        debug!(?path, "Writing README.md file…");
        let mut readme = File::create(path)?;
        readme.write_all(include_bytes!("dump_db/readme_for_tarball.md"))?;
        Ok(())
    }

    fn add_metadata(&self) -> Result<(), PerformError> {
        #[derive(Serialize)]
        struct Metadata<'a> {
            timestamp: &'a chrono::DateTime<chrono::Utc>,
            crates_io_commit: String,
        }
        let metadata = Metadata {
            timestamp: &self.timestamp,
            crates_io_commit: dotenv::var("HEROKU_SLUG_COMMIT")
                .unwrap_or_else(|_| "unknown".to_owned()),
        };
        let path = self.export_dir.join("metadata.json");
        debug!(?path, "Writing metadata.json file…");
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &metadata)?;
        Ok(())
    }

    pub fn dump_schema(&self, database_url: &str) -> Result<(), PerformError> {
        let path = self.export_dir.join("schema.sql");
        debug!(?path, "Writing schema.sql file…");
        let schema_sql = File::create(path)?;

        let status = std::process::Command::new("pg_dump")
            .arg("--schema-only")
            .arg("--no-owner")
            .arg("--no-acl")
            .arg(database_url)
            .stdout(schema_sql)
            .spawn()?
            .wait()?;
        if !status.success() {
            return Err("pg_dump did not finish successfully.".into());
        }
        Ok(())
    }

    pub fn dump_db(&self, database_url: &str) -> Result<(), PerformError> {
        debug!("Generating export.sql and import.sql files…");
        let export_script = self.export_dir.join("export.sql");
        let import_script = self.export_dir.join("import.sql");
        gen_scripts::gen_scripts(&export_script, &import_script)?;

        debug!("Filling data folder…");
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
    debug!(?script, "Running psql script…");
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
        return Err(format!("Error while executing psql: {stderr}").into());
    }
    if !output.status.success() {
        return Err("psql did not finish successfully.".into());
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
        let tar_top_dir = PathBuf::from(export_dir.file_name().unwrap());
        archive.append_dir(&tar_top_dir, export_dir)?;

        // Append readme, metadata, schemas.
        for entry in fs::read_dir(export_dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_file() {
                let name_in_tar = tar_top_dir.join(entry.file_name());
                archive.append_path_with_name(entry.path(), name_in_tar)?;
            }
        }

        // Append topologically sorted tables to make it possible to pipeline
        // importing with gz extraction.
        archive.append_dir(tar_top_dir.join("data"), export_dir.join("data"))?;
        for table in VisibilityConfig::get().topological_sort() {
            let csv_path = export_dir.join("data").join(table).with_extension("csv");
            if csv_path.exists() {
                let name_in_tar = tar_top_dir.join("data").join(table).with_extension("csv");
                archive.append_path_with_name(csv_path, name_in_tar)?;
            }
        }

        Ok(result)
    }

    fn upload(&self, target_name: &str, uploader: &Uploader) -> Result<u64, PerformError> {
        let client = reqwest::blocking::Client::new();
        let tarfile = File::open(&self.tarball_path)?;
        let content_length = tarfile.metadata()?.len();
        // TODO Figure out the correct content type.
        uploader.upload(
            &client,
            target_name,
            tarfile,
            content_length,
            "application/gzip",
            header::HeaderMap::new(),
        )?;
        Ok(content_length)
    }
}

impl Drop for DumpTarball {
    fn drop(&mut self) {
        std::fs::remove_file(&self.tarball_path).unwrap();
    }
}

mod configuration;
mod gen_scripts;

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;
    use tar::Archive;

    #[test]
    fn test_dump_tarball() {
        let tempdir = tempfile::Builder::new()
            .prefix("DumpTarball")
            .tempdir()
            .unwrap();
        let p = tempdir.path().join("0000-00-00");

        fs::create_dir(&p).unwrap();
        fs::write(p.join("README.md"), "# crates.io Database Dump\n").unwrap();
        fs::create_dir(p.join("data")).unwrap();
        fs::write(p.join("data").join("crates.csv"), "").unwrap();
        fs::write(p.join("data").join("crate_owners.csv"), "").unwrap();
        fs::write(p.join("data").join("users.csv"), "").unwrap();

        let tarball = DumpTarball::create(&p).unwrap();
        let gz = GzDecoder::new(File::open(&tarball.tarball_path).unwrap());
        let mut tar = Archive::new(gz);

        for (i, entry) in tar.entries().unwrap().enumerate() {
            let entry = entry.unwrap();
            let expected_path = Path::new(match i {
                0 => "0000-00-00",
                1 => "0000-00-00/README.md",
                2 => "0000-00-00/data",
                3 => "0000-00-00/data/crates.csv",
                4 => "0000-00-00/data/users.csv", // alphabetically after crates.csv
                5 => "0000-00-00/data/crate_owners.csv", // depends on crates.csv and users.csv
                _ => panic!("unexpected extra tar entry: {:?}", entry.path()),
            });
            assert_eq!(entry.path().unwrap(), expected_path, "entry {}", i);
        }
    }
}
