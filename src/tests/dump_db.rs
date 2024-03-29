use crates_io::worker::jobs::dump_db;
use crates_io_test_db::TestDatabase;

#[test]
fn dump_db_and_reimport_dump() {
    crates_io::util::tracing::init_for_test();

    let db_one = TestDatabase::new();

    // TODO prefill database with some data

    let directory = dump_db::DumpDirectory::create().unwrap();
    directory.populate(db_one.url()).unwrap();

    let db_two = TestDatabase::new();

    let import_script = directory.export_dir.join("import.sql");
    dump_db::run_psql(&import_script, db_two.url()).unwrap();

    // TODO: Consistency checks on the re-imported data?
}
