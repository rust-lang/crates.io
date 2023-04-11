use crate::util::FreshSchema;
use cargo_registry::worker::dump_db;

#[test]
fn dump_db_and_reimport_dump() {
    cargo_registry::util::tracing::init_for_test();

    let database_url = crate::env("TEST_DATABASE_URL");

    // TODO prefill database with some data

    let directory = dump_db::DumpDirectory::create().unwrap();
    directory.populate(&database_url).unwrap();

    let schema = FreshSchema::new(&database_url);

    let import_script = directory.export_dir.join("import.sql");
    dump_db::run_psql(&import_script, schema.database_url()).unwrap();

    // TODO: Consistency checks on the re-imported data?
}
