#![doc = include_str!("../README.md")]

use crates_io_env_vars::required_var_parsed;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sql_query;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use diesel_migrations::{FileBasedMigrations, MigrationHarness};
use rand::RngExt;
use std::sync::LazyLock;
use std::time::Duration;
use tracing::{debug, instrument};
use url::Url;

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

struct TemplateDatabase {
    base_url: Url,
    pool: Pool<ConnectionManager<PgConnection>>,
    template_name: String,
    prefix: String,
}

impl TemplateDatabase {
    #[instrument]
    pub fn instance() -> &'static Self {
        static INSTANCE: LazyLock<TemplateDatabase> = LazyLock::new(TemplateDatabase::new);
        &INSTANCE
    }

    #[instrument]
    fn new() -> Self {
        let base_url: Url = required_var_parsed("TEST_DATABASE_URL").unwrap();

        let prefix = base_url.path().strip_prefix('/');
        let prefix = prefix.expect("failed to parse database name").to_string();

        // Having only a single database management connection was causing
        // contention, so this is using a connection pool to reduce unnecessary
        // waiting times for the tests.
        let pool = Pool::builder()
            .connection_timeout(CONNECTION_TIMEOUT)
            .max_size(10)
            .min_idle(Some(0))
            .build_unchecked(ConnectionManager::new(base_url.as_ref()));

        // Get a connection from the pool, and create the template database
        let mut conn = pool.get().expect("failed to connect to the database");

        let template_name = format!("{prefix}_template");
        create_template_database(&template_name, &mut conn)
            .expect("failed to create template database");

        let mut template_url = base_url.clone();
        template_url.set_path(&format!("/{template_name}"));

        // Connect to the template database and run the migrations
        let mut template_conn =
            connect(template_url.as_ref()).expect("failed to connect to the template database");
        run_migrations(&mut template_conn)
            .expect("failed to run migrations on the template database");

        TemplateDatabase {
            base_url,
            pool,
            template_name,
            prefix,
        }
    }

    #[instrument(skip(self))]
    fn get_connection(&self) -> PooledConnection<ConnectionManager<PgConnection>> {
        self.pool.get().expect("Failed to get database connection")
    }
}

pub struct TestDatabase {
    name: String,
    url: Url,
    pool: Option<Pool<ConnectionManager<PgConnection>>>,
}

impl TestDatabase {
    /// Creates a new Postgres database based on a template with all of the
    /// migrations already applied. Once the `TestDatabase` instance is dropped,
    /// the database is automatically deleted.
    #[allow(clippy::new_without_default)]
    #[instrument]
    pub fn new() -> TestDatabase {
        Self::new_inner(|name, conn| {
            let template = TemplateDatabase::instance();
            create_database_from_template(name, &template.template_name, conn)
        })
    }

    /// Creates a new Postgres database. Once the `TestDatabase` instance is
    /// dropped, the database is automatically deleted.
    #[instrument]
    pub fn empty() -> TestDatabase {
        Self::new_inner(create_database)
    }

    fn new_inner(f: impl Fn(&str, &mut PgConnection) -> QueryResult<()>) -> TestDatabase {
        let template = TemplateDatabase::instance();

        let name = format!("{}_{}", template.prefix, generate_name().to_lowercase());

        let mut conn = template.get_connection();
        f(&name, &mut conn).expect("Failed to create test database");

        let mut url = template.base_url.clone();
        url.set_path(&format!("/{name}"));

        let pool = Pool::builder()
            .connection_timeout(CONNECTION_TIMEOUT)
            .min_idle(Some(0))
            .build_unchecked(ConnectionManager::new(url.as_ref()));

        let pool = Some(pool);
        TestDatabase { name, url, pool }
    }

    pub fn url(&self) -> &str {
        self.url.as_ref()
    }

    #[instrument(skip(self))]
    pub fn connect(&self) -> PooledConnection<ConnectionManager<PgConnection>> {
        self.pool
            .as_ref()
            .unwrap()
            .get()
            .expect("Failed to get database connection")
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
        // Essentially `drop(self.pool)` to make sure any connections to the
        // test database have been disconnected before dropping the database
        // itself.
        self.pool = None;

        let mut conn = TemplateDatabase::instance().get_connection();
        drop_database(&self.name, &mut conn).expect("failed to drop test database");
    }
}

#[instrument]
fn connect(database_url: &str) -> ConnectionResult<PgConnection> {
    debug!("Connecting to database…");
    PgConnection::establish(database_url)
}

#[instrument(skip(conn))]
fn create_database(name: &str, conn: &mut PgConnection) -> QueryResult<()> {
    debug!("Creating new database…");
    sql_query(format!("CREATE DATABASE {name}")).execute(conn)?;
    Ok(())
}

#[instrument(skip(conn))]
fn create_template_database(name: &str, conn: &mut PgConnection) -> QueryResult<()> {
    table! {
        pg_database (datname) {
            datname -> Text,
        }
    }

    debug!("Checking if template database already exists…");
    let count: i64 = pg_database::table
        .count()
        .filter(pg_database::datname.eq(name))
        .get_result(conn)?;

    if count == 0 {
        create_database(name, conn)?;
    } else {
        debug!(%count, "Skipping template database creation");
    }

    Ok(())
}

#[instrument(skip(conn))]
fn create_database_from_template(
    name: &str,
    template_name: &str,
    conn: &mut PgConnection,
) -> QueryResult<()> {
    debug!("Creating new test database from template…");
    sql_query(format!("CREATE DATABASE {name} TEMPLATE {template_name}")).execute(conn)?;
    Ok(())
}

#[instrument(skip(conn))]
fn drop_database(name: &str, conn: &mut PgConnection) -> QueryResult<()> {
    debug!("Dropping database…");
    sql_query(format!("DROP DATABASE {name} WITH (FORCE)")).execute(conn)?;
    Ok(())
}

#[instrument(skip(conn))]
fn run_migrations(conn: &mut PgConnection) -> diesel::migration::Result<()> {
    debug!("Running pending database migrations…");
    let migrations = FileBasedMigrations::find_migrations_directory()?;
    conn.run_pending_migrations(migrations)?;
    Ok(())
}

fn generate_name() -> String {
    let mut rng = rand::rng();
    std::iter::repeat(())
        .map(|_| rng.sample(rand::distr::Alphanumeric) as char)
        .take(16)
        .collect()
}
