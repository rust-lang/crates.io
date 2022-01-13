use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel_migrations::{find_migrations_directory, run_pending_migrations_in_directory};
use rand::Rng;

pub(crate) struct FreshSchema {
    database_url: String,
    schema_name: String,
    management_conn: PgConnection,
}

impl FreshSchema {
    pub(crate) fn new(database_url: &str) -> Self {
        let schema_name = generate_schema_name();

        let conn = PgConnection::establish(database_url).expect("can't connect to the test db");
        conn.batch_execute(&format!(
            "
                DROP SCHEMA IF EXISTS {schema_name} CASCADE;
                CREATE SCHEMA {schema_name};
                SET search_path TO {schema_name}, public;
            "
        ))
        .expect("failed to initialize schema");

        let migrations_dir = find_migrations_directory().unwrap();
        run_pending_migrations_in_directory(&conn, &migrations_dir, &mut std::io::sink())
            .expect("failed to run migrations on the test schema");

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
        self.management_conn
            .batch_execute(&format!("DROP SCHEMA {} CASCADE;", self.schema_name))
            .expect("failed to drop the test schema");
    }
}

fn generate_schema_name() -> String {
    let mut rng = rand::thread_rng();
    let random_string: String = std::iter::repeat(())
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .take(16)
        .collect();
    format!("cratesio_test_{random_string}")
}
