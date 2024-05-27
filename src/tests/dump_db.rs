use crates_io::worker::jobs::dump_db;
use crates_io_test_db::TestDatabase;
use once_cell::sync::Lazy;
use std::sync::Mutex;

/// Mutex to ensure that only one test is dumping the database at a time, since
/// the dump directory is shared between all invocations of the background job.
static DUMP_DIR_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[test]
fn dump_db_and_reimport_dump() {
    let _guard = DUMP_DIR_MUTEX.lock();

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
