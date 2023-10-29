use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel_migrations::{FileBasedMigrations, MigrationHarness};
use rand::Rng;
use tracing::instrument;

pub(crate) struct FreshSchema {
    database_url: String,
    schema_name: String,
    management_conn: PgConnection,
}

impl FreshSchema {
    pub(crate) fn new(database_url: &str) -> Self {
        let schema_name = generate_schema_name();

        let mut conn = connect(database_url).expect("can't connect to the test db");
        create_schema(&schema_name, &mut conn).expect("failed to initialize schema");
        run_migrations(&mut conn).expect("failed to run migrations on the test schema");

        let database_url = url::Url::parse_with_params(
            database_url,
            &[("options", format!("--search_path={schema_name},public"))],
        )
        .unwrap()
        .to_string();

        Self {
            database_url,
            schema_name,
            management_conn: conn,
        }
    }

    pub(crate) fn database_url(&self) -> &str {
        &self.database_url
    }
}

impl Drop for FreshSchema {
    fn drop(&mut self) {
        drop_schema(&self.schema_name, &mut self.management_conn)
            .expect("failed to drop the test schema");
    }
}

#[instrument]
fn connect(database_url: &str) -> ConnectionResult<PgConnection> {
    PgConnection::establish(database_url)
}

#[instrument(skip(conn))]
fn create_schema(schema_name: &str, conn: &mut PgConnection) -> QueryResult<()> {
    conn.batch_execute(&format!(
        "
            DROP SCHEMA IF EXISTS {schema_name} CASCADE;
            CREATE SCHEMA {schema_name};
            SET search_path TO {schema_name}, public;
        "
    ))
}

#[instrument(skip(conn))]
fn drop_schema(schema_name: &str, conn: &mut PgConnection) -> QueryResult<()> {
    conn.batch_execute(&format!("DROP SCHEMA {schema_name} CASCADE;"))
}

#[instrument(skip(conn))]
fn run_migrations(conn: &mut PgConnection) -> diesel::migration::Result<()> {
    let migrations = FileBasedMigrations::find_migrations_directory()?;
    conn.run_pending_migrations(migrations)?;
    Ok(())
}

fn generate_schema_name() -> String {
    let mut rng = rand::thread_rng();
    let random_string: String = std::iter::repeat(())
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .take(16)
        .collect();
    format!("cratesio_test_{random_string}")
}
