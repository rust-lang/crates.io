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
    directory.dump_db(&database_url).unwrap();

    let import_script = directory.export_dir.join("import.sql");
    let schema = TemporarySchema::create(database_url, "test_db_dump");
    diesel_migrations::run_pending_migrations(&schema.connection).unwrap();
    dump_db::run_psql(&import_script, &schema.database_url).unwrap();
}

struct TemporarySchema {
    pub database_url: String,
    pub schema_name: String,
    pub connection: PgConnection,
}

impl TemporarySchema {
    fn create(database_url: String, schema_name: &str) -> Self {
        let params = &[("options", format!("--search_path={}", schema_name))];
        let database_url = url::Url::parse_with_params(&database_url, params)
            .unwrap()
            .into_string();
        let schema_name = schema_name.to_owned();
        let connection = PgConnection::establish(&database_url).unwrap();
        connection
            .batch_execute(&format!(
                r#"DROP SCHEMA IF EXISTS "{schema_name}" CASCADE;
                   CREATE SCHEMA "{schema_name}";
                   SET SESSION search_path TO "{schema_name}",public;"#,
                schema_name = schema_name,
            ))
            .unwrap();
        Self {
            database_url,
            schema_name,
            connection,
        }
    }
}

impl Drop for TemporarySchema {
    fn drop(&mut self) {
        self.connection
            .batch_execute(&format!(
                r#"SET SESSION search_path TO DEFAULT;
                   DROP SCHEMA {schema_name} CASCADE;"#,
                schema_name = self.schema_name,
            ))
            .unwrap();
    }
}
