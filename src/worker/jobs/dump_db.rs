use self::configuration::VisibilityConfig;
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use anyhow::{anyhow, Context};
use crates_io_worker::BackgroundJob;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
pub struct DumpDb {
    database_url: String,
}

impl DumpDb {
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
        }
    }
}

impl BackgroundJob for DumpDb {
    const JOB_NAME: &'static str = "dump_db";

    type Context = Arc<Environment>;

    /// Create CSV dumps of the public information in the database, wrap them in a
    /// tarball and upload to S3.
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        let target_name = "db-dump.tar.gz";
        let database_url = self.database_url.clone();

        let tarball = spawn_blocking(move || {
            let directory = DumpDirectory::create()?;

            info!("Begin exporting database");
            directory.populate(&database_url)?;

            let export_dir = directory.path();
            info!(path = ?export_dir, "Creating tarball");
            let prefix = PathBuf::from(directory.timestamp.format("%Y-%m-%d-%H%M%S").to_string());
            create_tarball(export_dir, &prefix)
        })
        .await?;

        info!("Uploading tarball");
        env.storage
            .upload_db_dump(target_name, tarball.path())
            .await?;
        info!("Database dump tarball uploaded");

        info!("Invalidating CDN caches");
        if let Some(cloudfront) = env.cloudfront() {
            if let Err(error) = cloudfront.invalidate(target_name).await {
                warn!("failed to invalidate CloudFront cache: {}", error);
            }
        }

        if let Some(fastly) = env.fastly() {
            if let Err(error) = fastly.invalidate(target_name).await {
                warn!("failed to invalidate Fastly cache: {}", error);
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

fn create_tarball(export_dir: &Path, prefix: &Path) -> anyhow::Result<tempfile::NamedTempFile> {
    debug!("Creating tarball file");
    let tempfile = tempfile::NamedTempFile::new()?;
    let encoder = flate2::write::GzEncoder::new(tempfile.as_file(), flate2::Compression::default());

    let mut archive = tar::Builder::new(encoder);

    debug!(path = ?prefix, "Appending directory to tarball");
    archive.append_dir(prefix, export_dir)?;

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
        let name_in_tar = prefix.join(file_name);
        debug!(name = ?name_in_tar, "Appending file to tarball");
        archive.append_path_with_name(path, name_in_tar)?;
    }

    // Append topologically sorted tables to make it possible to pipeline
    // importing with gz extraction.

    debug!("Sorting database tables");
    let visibility_config = VisibilityConfig::get();
    let sorted_tables = visibility_config.topological_sort();

    let path = prefix.join("data");
    debug!(?path, "Appending directory to tarball");
    archive.append_dir(path, export_dir.join("data"))?;
    for table in sorted_tables {
        let csv_path = export_dir.join("data").join(table).with_extension("csv");
        if csv_path.exists() {
            let name_in_tar = prefix.join("data").join(table).with_extension("csv");
            debug!(name = ?name_in_tar, "Appending file to tarball");
            archive.append_path_with_name(csv_path, name_in_tar)?;
        }
    }

    drop(archive);

    Ok(tempfile)
}

mod configuration;
mod gen_scripts;

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;
    use insta::assert_debug_snapshot;
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

        let tarball = create_tarball(p, &PathBuf::from("0000-00-00")).unwrap();
        let gz = GzDecoder::new(File::open(tarball.path()).unwrap());
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
    }
}
