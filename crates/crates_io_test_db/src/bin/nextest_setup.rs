use anyhow::Context;
use crates_io_env_vars::required_var_parsed;
use crates_io_test_db::prepare_template_db;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use url::Url;

/// Nextest setup script that prepares the shared `test_template` schema
/// once per `cargo nextest run` invocation and publishes the path to the
/// captured DDL via the `NEXTEST_ENV` file as
/// `CRATES_IO_TEST_DB_DDL_PATH=<path>`. Test binaries that depend on
/// `crates_io_test_db` pick up that env var and skip per-process
/// migrations and `pg_dump`.
fn main() -> anyhow::Result<()> {
    let base_url: Url = required_var_parsed("TEST_DATABASE_URL")?;
    let path = prepare_template_db(&base_url)?;

    let env_file = env::var("NEXTEST_ENV")
        .context("NEXTEST_ENV is not set (this binary must be run as a nextest setup script)")?;
    let mut file = OpenOptions::new().append(true).open(&env_file)?;
    writeln!(file, "CRATES_IO_TEST_DB_DDL_PATH={}", path.display())?;
    Ok(())
}
