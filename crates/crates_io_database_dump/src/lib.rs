#![doc = include_str!("../README.md")]

use anyhow::{Context, anyhow};
use serde::Serialize;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;
use zip::write::SimpleFileOptions;

mod configuration;
mod gen_scripts;

pub use configuration::VisibilityConfig;
pub use gen_scripts::gen_scripts;

/// Manage the export directory.
///
/// Create the directory, populate it with the psql scripts and CSV dumps, and
/// make sure it gets deleted again even in the case of an error.
#[derive(Debug)]
pub struct DumpDirectory {
    /// The temporary directory that contains the export directory.
    tempdir: tempfile::TempDir,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Optional directory containing the `pg_dump` and `psql` binaries to use.
    /// When `None`, the binaries are resolved via `PATH`.
    postgres_bin_dir: Option<PathBuf>,
}

impl DumpDirectory {
    pub fn create(postgres_bin_dir: Option<PathBuf>) -> anyhow::Result<Self> {
        debug!("Creating database dump folder…");
        let tempdir = tempfile::tempdir()?;
        let timestamp = chrono::Utc::now();

        Ok(Self {
            tempdir,
            timestamp,
            postgres_bin_dir,
        })
    }

    pub fn path(&self) -> &Path {
        self.tempdir.path()
    }

    /// Resolve the path of a PostgreSQL client binary, honoring the configured
    /// [`Self::postgres_bin_dir`] override. When no override is set the bare
    /// name is returned, which `Command::new` will resolve via `PATH`.
    fn pg_program(&self, name: &str) -> PathBuf {
        match &self.postgres_bin_dir {
            Some(dir) => dir.join(name),
            None => PathBuf::from(name),
        }
    }

    /// Generate the full export directory (README, metadata, `schema.sql`,
    /// `export.sql`/`import.sql`, and CSV data files) from the database at
    /// `database_url`. When `schema` is `Some`, the dump is restricted to that
    /// Postgres schema; when `None`, every schema in the database is dumped.
    /// Production callers pass `None`; the test harness passes the test schema
    /// so its `pg_dump` doesn't race with concurrent test schemas.
    pub fn populate(&self, database_url: &str, schema: Option<&str>) -> anyhow::Result<()> {
        self.add_readme()
            .context("Failed to write README.md file")?;

        self.add_metadata()
            .context("Failed to write metadata.json file")?;

        self.dump_schema(database_url, schema)
            .context("Failed to generate schema.sql file")?;

        self.dump_db(database_url)
            .context("Failed to create database dump")
    }

    fn add_readme(&self) -> anyhow::Result<()> {
        use std::io::Write;

        let path = self.path().join("README.md");
        debug!(?path, "Writing README.md file…");
        let mut readme = File::create(path)?;
        readme.write_all(include_bytes!("readme_for_tarball.md"))?;
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
            crates_io_commit: crates_io_version::commit()
                .ok()
                .flatten()
                .unwrap_or_else(|| "unknown".to_owned()),
        };
        let path = self.path().join("metadata.json");
        debug!(?path, "Writing metadata.json file…");
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &metadata)?;
        Ok(())
    }

    pub fn dump_schema(&self, database_url: &str, schema: Option<&str>) -> anyhow::Result<()> {
        let path = self.path().join("schema.sql");
        debug!(?path, "Writing schema.sql file…");
        let schema_sql =
            File::create(&path).with_context(|| format!("Failed to create {}", path.display()))?;

        let program = self.pg_program("pg_dump");

        let mut command = Command::new(&program);
        command
            .arg("--schema-only")
            .arg("--no-owner")
            .arg("--no-acl");

        if let Some(schema) = schema {
            command.arg(format!("--schema={schema}"));
        }

        let status = command
            .arg(database_url)
            .stdout(schema_sql)
            .spawn()
            .with_context(|| format!("Failed to run `{}` command", program.display()))?
            .wait()
            .with_context(|| format!("Failed to wait for `{}` to exit", program.display()))?;

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
        gen_scripts(&export_script, &import_script)
            .context("Failed to generate export/import scripts")?;

        debug!("Filling data folder…");
        fs::create_dir(self.path().join("data")).context("Failed to create `data` directory")?;

        self.run_psql(&export_script, database_url)
    }

    pub fn run_psql(&self, script: &Path, database_url: &str) -> anyhow::Result<()> {
        debug!(?script, "Running psql script…");
        let psql_script =
            File::open(script).with_context(|| format!("Failed to open {}", script.display()))?;

        let program = self.pg_program("psql");
        let psql = Command::new(&program)
            .arg("--no-psqlrc")
            .arg(database_url)
            .current_dir(script.parent().unwrap())
            .stdin(psql_script)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to run `{}` command", program.display()))?;

        let output = psql.wait_with_output().with_context(|| {
            format!("Failed to wait for `{}` command to exit", program.display())
        })?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("ERROR") {
            return Err(anyhow!("Error while executing psql: {stderr}"));
        }
        if !output.status.success() {
            return Err(anyhow!("psql did not finish successfully."));
        }
        Ok(())
    }
}

pub struct Archives {
    pub tar: tempfile::NamedTempFile,
    pub zip: tempfile::NamedTempFile,
}

pub fn create_archives(export_dir: &Path, tarball_prefix: &Path) -> anyhow::Result<Archives> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crates_io_env_vars::var_parsed;
    use crates_io_test_db::TestDatabase;
    use flate2::read::GzDecoder;
    use insta::{assert_debug_snapshot, assert_snapshot};
    use std::io::BufReader;
    use tar::Archive;

    fn postgres_bin_dir() -> Option<PathBuf> {
        var_parsed("POSTGRES_BIN_DIR").unwrap()
    }

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
        fs::write(p.join("data").join("users_v2.csv"), "").unwrap();

        let archives = create_archives(p, &PathBuf::from("0000-00-00")).unwrap();
        let gz = GzDecoder::new(File::open(archives.tar.path()).unwrap());
        let mut tar = Archive::new(gz);

        let entries = tar.entries().unwrap();
        let paths = entries
            .map(|entry| entry.unwrap().path().unwrap().display().to_string())
            .collect::<Vec<_>>();

        assert_debug_snapshot!(paths, @r#"
        [
            "0000-00-00",
            "0000-00-00/README.md",
            "0000-00-00/data",
            "0000-00-00/data/crates.csv",
            "0000-00-00/data/users_v2.csv",
            "0000-00-00/data/crate_owners.csv",
        ]
        "#);

        let file = File::open(archives.zip.path()).unwrap();
        let reader = BufReader::new(file);

        let archive = zip::ZipArchive::new(reader).unwrap();
        let zip_paths = archive.file_names().collect::<Vec<_>>();
        assert_debug_snapshot!(zip_paths, @r#"
        [
            "README.md",
            "data/",
            "data/crates.csv",
            "data/users_v2.csv",
            "data/crate_owners.csv",
        ]
        "#);
    }

    #[test]
    fn dump_db_and_reimport_dump() {
        use diesel::RunQueryDsl;
        use diesel::sql_query;

        let test_db = TestDatabase::new();

        // TODO prefill database with some data

        let directory = DumpDirectory::create(postgres_bin_dir()).unwrap();
        directory
            .populate(test_db.url(), Some(test_db.schema()))
            .unwrap();

        // Clear the schema so the dump's `CREATE SCHEMA` and `CREATE TABLE`
        // statements (qualified with `test_db.schema()`) have a fresh target.
        // The schema name in the URL's `search_path` resolves again as soon
        // as the dump recreates it. `test_db`'s `Drop` cleans up the
        // recreated schema at the end of the test.
        let mut conn = test_db.connect();
        sql_query(format!("DROP SCHEMA \"{}\" CASCADE", test_db.schema()))
            .execute(&mut conn)
            .unwrap();

        let schema_script = directory.path().join("schema.sql");
        directory.run_psql(&schema_script, test_db.url()).unwrap();

        let import_script = directory.path().join("import.sql");
        directory.run_psql(&import_script, test_db.url()).unwrap();

        // TODO: Consistency checks on the re-imported data?
    }

    #[test]
    fn test_sql_scripts() {
        let db = TestDatabase::new();

        let directory = DumpDirectory::create(postgres_bin_dir()).unwrap();
        directory.populate(db.url(), Some(db.schema())).unwrap();

        insta::glob!(directory.path(), "{import,export}.sql", |path| {
            let content = fs::read_to_string(path).unwrap();
            assert_snapshot!(content);
        });
    }
}
