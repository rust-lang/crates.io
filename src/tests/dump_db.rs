use cargo_registry::tasks::dump_db;
use diesel::{
    connection::{Connection, SimpleConnection},
    pg::PgConnection,
};

#[test]
fn dump_db_and_reimport_dump() {
    let database_url = crate::env("TEST_DATABASE_URL");

    // TODO prefill database with some data

    let directory = dump_db::DumpDirectory::create().unwrap();
    directory.populate(&database_url).unwrap();

    let schema = TemporarySchema::create(database_url, "test_db_dump");
    schema.run_migrations();

    let import_script = directory.export_dir.join("import.sql");
    dump_db::run_psql(&import_script, &schema.database_url).unwrap();

    // TODO: Consistency checks on the re-imported data?
}

struct TemporarySchema {
    pub database_url: String,
    pub schema_name: String,
    pub connection: PgConnection,
}

impl TemporarySchema {
    pub fn create(database_url: String, schema_name: &str) -> Self {
        let params = &[("options", format!("--search_path={},public", schema_name))];
        let database_url = url::Url::parse_with_params(&database_url, params)
            .unwrap()
            .into_string();
        let schema_name = schema_name.to_owned();
        let connection = PgConnection::establish(&database_url).unwrap();
        connection
            .batch_execute(&format!(
                r#"DROP SCHEMA IF EXISTS "{schema_name}" CASCADE;
                   CREATE SCHEMA "{schema_name}";"#,
                schema_name = schema_name,
            ))
            .unwrap();
        Self {
            database_url,
            schema_name,
            connection,
        }
    }

    pub fn run_migrations(&self) {
        use diesel_migrations::{find_migrations_directory, run_pending_migrations_in_directory};
        let migrations_dir = find_migrations_directory().unwrap();
        run_pending_migrations_in_directory(
            &self.connection,
            &migrations_dir,
            &mut std::io::sink(),
        )
        .unwrap();
    }
}

impl Drop for TemporarySchema {
    fn drop(&mut self) {
        self.connection
            .batch_execute(&format!(r#"DROP SCHEMA "{}" CASCADE;"#, self.schema_name))
            .unwrap();
    }
}
