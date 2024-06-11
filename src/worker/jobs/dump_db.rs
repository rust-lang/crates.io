use self::configuration::VisibilityConfig;
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use anyhow::{anyhow, Context};
use crates_io_worker::BackgroundJob;
use secrecy::ExposeSecret;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zip::write::SimpleFileOptions;

#[derive(Clone, Serialize, Deserialize)]
pub struct DumpDb;

impl BackgroundJob for DumpDb {
    const JOB_NAME: &'static str = "dump_db";

    type Context = Arc<Environment>;

    /// Create CSV dumps of the public information in the database, wrap them in a
    /// tarball and upload to S3.
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        const TAR_PATH: &str = "db-dump.tar.gz";
        const ZIP_PATH: &str = "db-dump.zip";

        let db_config = &env.config.db;
        let db_pool_config = db_config.replica.as_ref().unwrap_or(&db_config.primary);
        let database_url = db_pool_config.url.clone();

        let archives = spawn_blocking(move || {
            let directory = DumpDirectory::create()?;

            info!("Exporting database…");
            directory.populate(database_url.expose_secret())?;

            let export_dir = directory.path();
            info!(path = ?export_dir, "Creating tarball…");
            let tarball_prefix =
                PathBuf::from(directory.timestamp.format("%Y-%m-%d-%H%M%S").to_string());
            create_archives(export_dir, &tarball_prefix)
        })
        .await?;

        info!("Uploading tarball…");
        env.storage
            .upload_db_dump(TAR_PATH, archives.tar.path())
            .await?;
        info!("Database dump tarball uploaded");

        info!("Invalidating CDN caches…");
        if let Some(cloudfront) = env.cloudfront() {
            if let Err(error) = cloudfront.invalidate(TAR_PATH).await {
                warn!("Failed to invalidate CloudFront cache: {}", error);
            }
        }

        if let Some(fastly) = env.fastly() {
            if let Err(error) = fastly.invalidate(TAR_PATH).await {
                warn!("Failed to invalidate Fastly cache: {}", error);
            }
        }

        info!("Uploading zip file…");
        env.storage
            .upload_db_dump(ZIP_PATH, archives.zip.path())
            .await?;
        info!("Database dump zip file uploaded");

        info!("Invalidating CDN caches…");
        if let Some(cloudfront) = env.cloudfront() {
            if let Err(error) = cloudfront.invalidate(ZIP_PATH).await {
                warn!("Failed to invalidate CloudFront cache: {}", error);
            }
        }

        if let Some(fastly) = env.fastly() {
            if let Err(error) = fastly.invalidate(ZIP_PATH).await {
                warn!("Failed to invalidate Fastly cache: {}", error);
            }
        }

        Ok(())
    }
}

/// Manage the export directory.
///
/// Create the directory, populate it with the psql scripts and CSV dumps, and
/// make sure it gets deleted again even in the case of an error.
#[derive(Debug)]
pub struct DumpDirectory {
    /// The temporary directory that contains the export directory.
    tempdir: tempfile::TempDir,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl DumpDirectory {
    pub fn create() -> anyhow::Result<Self> {
        debug!("Creating database dump folder…");
        let tempdir = tempfile::tempdir()?;
        let timestamp = chrono::Utc::now();

        Ok(Self { tempdir, timestamp })
    }

    pub fn path(&self) -> &Path {
        self.tempdir.path()
    }

    pub fn populate(&self, database_url: &str) -> anyhow::Result<()> {
        self.add_readme()
            .context("Failed to write README.md file")?;

        self.add_metadata()
            .context("Failed to write metadata.json file")?;

        self.dump_schema(database_url)
            .context("Failed to generate schema.sql file")?;

        self.dump_db(database_url)
            .context("Failed to create database dump")
    }

    fn add_readme(&self) -> anyhow::Result<()> {
        use std::io::Write;

        let path = self.path().join("README.md");
        debug!(?path, "Writing README.md file…");
        let mut readme = File::create(path)?;
        readme.write_all(include_bytes!("dump_db/readme_for_tarball.md"))?;
        Ok(())
    }

    fn add_metadata(&self) -> anyhow::Result<()> {
        #[derive(Serialize)]
        struct Metadata<'a> {
            timestamp: &'a chrono::DateTime<chrono::Utc>,
            crates_io_commit: String,
        }
        let metadata = Metadata {
            timestamp: &self.timestamp,
            crates_io_commit: dotenvy::var("HEROKU_SLUG_COMMIT")
                .unwrap_or_else(|_| "unknown".to_owned()),
        };
        let path = self.path().join("metadata.json");
        debug!(?path, "Writing metadata.json file…");
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &metadata)?;
        Ok(())
    }

    pub fn dump_schema(&self, database_url: &str) -> anyhow::Result<()> {
        let path = self.path().join("schema.sql");
        debug!(?path, "Writing schema.sql file…");
        let schema_sql =
            File::create(&path).with_context(|| format!("Failed to create {}", path.display()))?;

        let status = std::process::Command::new("pg_dump")
            .arg("--schema-only")
            .arg("--no-owner")
            .arg("--no-acl")
            .arg(database_url)
            .stdout(schema_sql)
            .spawn()
            .context("Failed to run `pg_dump` command")?
            .wait()
            .context("Failed to wait for `pg_dump` to exit")?;

        if !status.success() {
            return Err(anyhow!(
                "pg_dump did not finish successfully (exit code: {}).",
                status
            ));
        }

        Ok(())
    }

    pub fn dump_db(&self, database_url: &str) -> anyhow::Result<()> {
        debug!("Generating export.sql and import.sql files…");
        let export_script = self.path().join("export.sql");
        let import_script = self.path().join("import.sql");
        gen_scripts::gen_scripts(&export_script, &import_script)
            .context("Failed to generate export/import scripts")?;

        debug!("Filling data folder…");
        fs::create_dir(self.path().join("data")).context("Failed to create `data` directory")?;

        run_psql(&export_script, database_url)
    }
}

pub fn run_psql(script: &Path, database_url: &str) -> anyhow::Result<()> {
    debug!(?script, "Running psql script…");
    let psql_script =
        File::open(script).with_context(|| format!("Failed to open {}", script.display()))?;

    let psql = std::process::Command::new("psql")
        .arg(database_url)
        .current_dir(script.parent().unwrap())
        .stdin(psql_script)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to run psql command")?;

    let output = psql
        .wait_with_output()
        .context("Failed to wait for psql command to exit")?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("ERROR") {
        return Err(anyhow!("Error while executing psql: {stderr}"));
    }
    if !output.status.success() {
        return Err(anyhow!("psql did not finish successfully."));
    }
    Ok(())
}

struct Archives {
    tar: tempfile::NamedTempFile,
    zip: tempfile::NamedTempFile,
}

fn create_archives(export_dir: &Path, tarball_prefix: &Path) -> anyhow::Result<Archives> {
    debug!("Creating tarball file…");
    let tar_tempfile = tempfile::NamedTempFile::new()?;
    let encoder =
        flate2::write::GzEncoder::new(tar_tempfile.as_file(), flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);

    debug!("Creating zip file…");
    let zip_tempfile = tempfile::NamedTempFile::new()?;
    let mut zip = zip::ZipWriter::new(zip_tempfile.as_file());

    debug!("Appending `{tarball_prefix:?}` directory to tarball…");
    tar.append_dir(tarball_prefix, export_dir)?;

    // Append readme, metadata, schemas.
    let mut paths = Vec::new();
    for entry in fs::read_dir(export_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_file() {
            paths.push((entry.path(), entry.file_name()));
        }
    }
    // Sort paths to make the tarball deterministic.
    paths.sort();
    for (path, file_name) in paths {
        let name = tarball_prefix.join(&file_name);
        debug!("Appending `{name:?}` file to tarball…");
        tar.append_path_with_name(&path, name)?;

        debug!("Appending `{file_name:?}` file to zip file…");
        zip.start_file_from_path(&file_name, SimpleFileOptions::default())?;
        std::io::copy(&mut File::open(path)?, &mut zip)?;
    }

    // Append topologically sorted tables to make it possible to pipeline
    // importing with gz extraction.

    debug!("Sorting database tables");
    let visibility_config = VisibilityConfig::get();
    let sorted_tables = visibility_config.topological_sort();

    let path = tarball_prefix.join("data");
    debug!("Appending `data` directory to tarball…");
    tar.append_dir(path, export_dir.join("data"))?;

    debug!("Appending `data` directory to zip file…");
    zip.add_directory("data", SimpleFileOptions::default())?;

    for table in sorted_tables {
        let csv_path = export_dir.join("data").join(table).with_extension("csv");
        if csv_path.exists() {
            let name = tarball_prefix
                .join("data")
                .join(table)
                .with_extension("csv");
            debug!("Appending `{name:?}` file to tarball…");
            tar.append_path_with_name(&csv_path, name)?;

            let name = PathBuf::from("data").join(table).with_extension("csv");
            debug!("Appending `{name:?}` file to zip file…");
            zip.start_file_from_path(&name, SimpleFileOptions::default())?;
            std::io::copy(&mut File::open(csv_path)?, &mut zip)?;
        }
    }

    drop(tar);
    zip.finish()?;

    Ok(Archives {
        tar: tar_tempfile,
        zip: zip_tempfile,
    })
}

mod configuration;
mod gen_scripts;

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;
    use insta::assert_debug_snapshot;
    use std::io::BufReader;
    use tar::Archive;

    #[test]
    fn test_dump_tarball() {
        let tempdir = tempfile::Builder::new()
            .prefix("DumpTarball")
            .tempdir()
            .unwrap();
        let p = tempdir.path();

        fs::write(p.join("README.md"), "# crates.io Database Dump\n").unwrap();
        fs::create_dir(p.join("data")).unwrap();
        fs::write(p.join("data").join("crates.csv"), "").unwrap();
        fs::write(p.join("data").join("crate_owners.csv"), "").unwrap();
        fs::write(p.join("data").join("users.csv"), "").unwrap();

        let archives = create_archives(p, &PathBuf::from("0000-00-00")).unwrap();
        let gz = GzDecoder::new(File::open(archives.tar.path()).unwrap());
        let mut tar = Archive::new(gz);

        let entries = tar.entries().unwrap();
        let paths = entries
            .map(|entry| entry.unwrap().path().unwrap().display().to_string())
            .collect::<Vec<_>>();

        assert_debug_snapshot!(paths, @r###"
        [
            "0000-00-00",
            "0000-00-00/README.md",
            "0000-00-00/data",
            "0000-00-00/data/crates.csv",
            "0000-00-00/data/users.csv",
            "0000-00-00/data/crate_owners.csv",
        ]
        "###);

        let file = File::open(archives.zip.path()).unwrap();
        let reader = BufReader::new(file);

        let archive = zip::ZipArchive::new(reader).unwrap();
        let zip_paths = archive.file_names().collect::<Vec<_>>();
        assert_debug_snapshot!(zip_paths, @r###"
        [
            "README.md",
            "data/",
            "data/crates.csv",
            "data/users.csv",
            "data/crate_owners.csv",
        ]
        "###);
    }
}
