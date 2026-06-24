#![doc = include_str!("../README.md")]

use crates_io_env_vars::{required_var_parsed, var_parsed};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sql_query;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use diesel_migrations::{FileBasedMigrations, MigrationHarness};
use rand::RngExt;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;
use std::time::Duration;
use tempfile::NamedTempFile;
use tracing::{debug, instrument};
use url::Url;

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

/// Prefix for per-test schemas, followed by a 16-character random suffix.
/// Used both when allocating new schemas and when sweeping up leftovers from
/// crashed prior runs.
const TEST_SCHEMA_PREFIX: &str = "test";

/// Schema that holds the migrated structure all per-test schemas are cloned
/// from. Created once per process.
const TEMPLATE_SCHEMA: &str = "test_template";

/// Management state shared by every `TestDatabase` in the process.
///
/// On first use we ensure the `test_template` schema exists, run all pending
/// migrations into it, and capture the resulting DDL via `pg_dump`. Each
/// subsequent `TestDatabase::new()` creates a fresh schema and replays that
/// captured DDL with the template schema name rewritten to the test schema
/// name, sidestepping the per-test migration-framework overhead.
struct Management {
    base_url: Url,
    pool: Pool<ConnectionManager<PgConnection>>,
    /// DDL captured from the template schema, with the template schema's
    /// own qualified references intact (e.g. `test_template.crates`). Per-test
    /// replay substitutes the template name for the test schema name.
    template_ddl: String,
}

impl Management {
    #[instrument]
    pub fn instance() -> &'static Self {
        static INSTANCE: LazyLock<Management> = LazyLock::new(Management::new);
        &INSTANCE
    }

    #[instrument]
    fn new() -> Self {
        let base_url: Url = required_var_parsed("TEST_DATABASE_URL").unwrap();

        // Under `cargo nextest`, the setup binary in this crate prepares the
        // template schema once and publishes the DDL path via this env var.
        // Plain `cargo test` leaves it unset and each process prepares its
        // own.
        let ddl_path: PathBuf = var_parsed("CRATES_IO_TEST_DB_DDL_PATH")
            .expect("invalid CRATES_IO_TEST_DB_DDL_PATH")
            .unwrap_or_else(|| {
                prepare_template_db(&base_url).expect("failed to prepare template DB")
            });

        let template_ddl = fs::read_to_string(&ddl_path).expect("failed to read template DDL file");

        let pool = Pool::builder()
            .connection_timeout(CONNECTION_TIMEOUT)
            .max_size(10)
            .min_idle(Some(0))
            .build_unchecked(ConnectionManager::new(base_url.as_ref()));

        Management {
            base_url,
            pool,
            template_ddl,
        }
    }

    #[instrument(skip(self))]
    fn get_connection(&self) -> PooledConnection<ConnectionManager<PgConnection>> {
        self.pool.get().expect("Failed to get database connection")
    }

    /// Generates a random schema name and builds the `TestDatabase` struct
    /// around it. The schema itself is not created here; callers are
    /// responsible for either issuing `CREATE SCHEMA` (see
    /// [`TestDatabase::empty`]) or replaying DDL that contains a `CREATE
    /// SCHEMA` statement (see [`TestDatabase::new`]).
    fn allocate(&self) -> TestDatabase {
        let schema = format!("{TEST_SCHEMA_PREFIX}_{}", generate_name().to_lowercase());
        let url = url_with_search_path(&self.base_url, &schema);

        TestDatabase { schema, url }
    }
}

pub struct TestDatabase {
    schema: String,
    /// Base URL for the test database with `options=--search_path=<schema>,public`
    /// appended, so any pool or one-shot connection opened against this URL
    /// is automatically scoped to the test's schema.
    url: Url,
}

impl TestDatabase {
    /// Creates a new schema inside the test database, populated by replaying
    /// the captured template DDL. The schema (and everything in it) is
    /// dropped when this `TestDatabase` is dropped.
    #[allow(clippy::new_without_default)]
    #[instrument]
    pub fn new() -> TestDatabase {
        let management = Management::instance();
        let test_db = management.allocate();

        let ddl = management
            .template_ddl
            .replace(TEMPLATE_SCHEMA, &test_db.schema);

        let mut conn = management.get_connection();
        conn.batch_execute(&ddl)
            .expect("failed to replay template DDL into test schema");

        test_db
    }

    /// Creates a new schema inside the test database without populating it.
    /// The schema is dropped when this `TestDatabase` is dropped.
    #[instrument]
    pub fn empty() -> TestDatabase {
        let management = Management::instance();
        let test_db = management.allocate();

        let mut conn = management.get_connection();
        create_schema(&test_db.schema, &mut conn).expect("Failed to create test schema");

        test_db
    }

    /// URL pointing at the test database, with the test schema baked in as a
    /// connection option (`options=--search_path=<schema>,public`). Any pool
    /// or one-shot connection opened against this URL is automatically scoped
    /// to the test's schema.
    pub fn url(&self) -> &str {
        self.url.as_ref()
    }

    /// Name of the schema this `TestDatabase` owns.
    pub fn schema(&self) -> &str {
        &self.schema
    }

    #[instrument(skip(self))]
    pub fn connect(&self) -> PgConnection {
        PgConnection::establish(self.url()).expect("Failed to connect to database")
    }

    #[instrument(skip(self))]
    pub async fn async_connect(&self) -> AsyncPgConnection {
        AsyncPgConnection::establish(self.url())
            .await
            .expect("Failed to connect to database")
    }
}

impl Drop for TestDatabase {
    #[instrument(skip(self))]
    fn drop(&mut self) {
        let mut conn = Management::instance().get_connection();
        drop_schema(&self.schema, &mut conn).expect("failed to drop test schema");
    }
}

/// Prepares the `test_template` schema and writes its DDL to a persistent
/// temporary file.
///
/// This sweeps any leftover per-test schemas from a prior crashed run,
/// ensures `test_template` exists with all pending migrations applied, and
/// dumps the schema via `pg_dump` into a file ready for
/// `conn.batch_execute()`. The returned path points at a file kept past
/// the function's lifetime via [`NamedTempFile::keep`]. Callers are
/// responsible for reading it.
#[instrument]
pub fn prepare_template_db(base_url: &Url) -> anyhow::Result<PathBuf> {
    let mut conn = connect(base_url.as_ref())?;

    // Drop any leftover test schemas from previous runs that crashed
    // without dropping their schema. This also drops any extensions
    // that were installed inside those schemas, freeing their
    // database-level registration so the migrations can re-install
    // them in `public`.
    cleanup_leftover_schemas(&mut conn)?;

    sql_query(format!("CREATE SCHEMA IF NOT EXISTS \"{TEMPLATE_SCHEMA}\"")).execute(&mut conn)?;

    // Apply any pending migrations to the template schema. Diesel skips
    // already-applied migrations, so subsequent process starts only pay
    // for newly-added migrations.
    sql_query(format!("SET search_path TO \"{TEMPLATE_SCHEMA}\", public")).execute(&mut conn)?;

    run_migrations(&mut conn).map_err(anyhow::Error::msg)?;

    drop(conn);

    let mut tempfile = NamedTempFile::new()?;
    capture_template_ddl(base_url, &mut tempfile)?;
    let (_, path) = tempfile.keep()?;

    Ok(path)
}

/// Drops any schema whose name matches `test_<16-alphanumeric-chars>`. These
/// are leftover test schemas from a prior run that crashed before its `Drop`
/// impl could clean up. Each `DROP SCHEMA … CASCADE` also drops any extension
/// that happened to be installed inside the leftover schema.
#[instrument(skip(conn))]
fn cleanup_leftover_schemas(conn: &mut PgConnection) -> QueryResult<()> {
    let leftovers: Vec<Schema> = diesel::sql_query(
        "SELECT schema_name FROM information_schema.schemata \
         WHERE schema_name ~ '^test_[a-z0-9]{16}$'",
    )
    .load(conn)?;

    for Schema { schema_name } in leftovers {
        debug!(name = %schema_name, "Dropping leftover test schema");
        sql_query(format!("DROP SCHEMA \"{schema_name}\" CASCADE")).execute(conn)?;
    }

    Ok(())
}

#[derive(diesel::QueryableByName)]
struct Schema {
    #[diesel(sql_type = diesel::sql_types::Text)]
    schema_name: String,
}

#[instrument]
fn connect(database_url: &str) -> ConnectionResult<PgConnection> {
    debug!("Connecting to database…");
    PgConnection::establish(database_url)
}

#[instrument(skip(conn))]
fn create_schema(name: &str, conn: &mut PgConnection) -> QueryResult<()> {
    debug!("Creating new test schema…");
    sql_query(format!("CREATE SCHEMA \"{name}\"")).execute(conn)?;
    Ok(())
}

#[instrument(skip(conn))]
fn drop_schema(name: &str, conn: &mut PgConnection) -> QueryResult<()> {
    debug!("Dropping test schema…");
    // `IF EXISTS` so that a `TestDatabase` whose `new()` panicked partway
    // through the DDL replay (before `CREATE SCHEMA` ran) still drops cleanly.
    sql_query(format!("DROP SCHEMA IF EXISTS \"{name}\" CASCADE")).execute(conn)?;
    Ok(())
}

#[instrument(skip(conn))]
fn run_migrations(conn: &mut PgConnection) -> diesel::migration::Result<()> {
    debug!("Running pending database migrations…");
    let migrations = FileBasedMigrations::find_migrations_directory()?;
    conn.run_pending_migrations(migrations)?;
    Ok(())
}

/// Shells out to `pg_dump --schema=<template>` and writes the captured DDL
/// as SQL ready for `batch_execute` into `out`.
///
/// `pg_dump`'s plain-text output targets `psql`, not the backend. To make
/// it backend-safe:
///
/// - `--inserts` avoids the `COPY … FROM stdin … \.` data framing that
///   `batch_execute` can't run.
/// - Lines starting with `\` (psql meta-commands like `\restrict`/
///   `\unrestrict`, emitted by `pg_dump` 17+) are stripped.
/// - SETs for configuration that `pg_dump` 17+ emit that PostgreSQL 16 can't
///   handle.
/// - A trailing `RESET ALL` clears any session state the preamble's
///   `SET` / `set_config('search_path', …)` calls left behind so it
///   doesn't leak to the next checkout of the pooled connection.
#[instrument(skip(out))]
fn capture_template_ddl(base_url: &Url, out: &mut impl Write) -> anyhow::Result<()> {
    let pg_dump = match var_parsed::<PathBuf>("POSTGRES_BIN_DIR")? {
        Some(dir) => dir.join("pg_dump"),
        None => PathBuf::from("pg_dump"),
    };
    debug!(pg_dump = %pg_dump.display(), "Capturing template schema DDL via pg_dump…");
    let output = Command::new(&pg_dump)
        .arg("--no-owner")
        .arg("--no-acl")
        .arg("--inserts")
        .arg(format!("--schema={TEMPLATE_SCHEMA}"))
        .arg(base_url.as_ref())
        .output()
        .map_err(|err| anyhow::anyhow!("failed to run `pg_dump`: {err}"))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "pg_dump did not finish successfully (exit code: {}). stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let raw = std::str::from_utf8(&output.stdout)
        .map_err(|err| anyhow::anyhow!("pg_dump produced non-UTF-8 output: {err}"))?;

    for line in raw.lines() {
        if line.starts_with('\\') {
            continue;
        }
        // pg_dump 17+ will generate a SET transaction_timeout statement that
        // PostgreSQL 16 chokes on.
        if line.trim_start().starts_with("SET transaction_timeout") {
            continue;
        }
        writeln!(out, "{line}")?;
    }

    writeln!(out, "\nRESET ALL;")?;

    Ok(())
}

/// Returns a copy of `base_url` with `options=--search_path=<schema>,public`
/// appended to its query string. Postgres honors this `options` parameter when
/// the connection is established, so every connection through the resulting
/// URL starts with the test schema active.
///
/// The `--name=value` form (rather than the more common `-c name=value`) keeps
/// the value space-free, which lets `url`'s form-encoding `append_pair()` do
/// the right thing — libpq does not accept `+` as a space in URL query
/// strings.
fn url_with_search_path(base_url: &Url, schema: &str) -> Url {
    let mut url = base_url.clone();
    url.query_pairs_mut()
        .append_pair("options", &format!("--search_path={schema},public"));
    url
}

fn generate_name() -> String {
    let mut rng = rand::rng();
    std::iter::repeat(())
        .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
        .take(16)
        .collect()
}
