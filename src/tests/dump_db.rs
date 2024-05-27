use crate::builders::CrateBuilder;
use crate::util::TestApp;
use bytes::Buf;
use crates_io::worker::jobs::{dump_db, DumpDb};
use crates_io_test_db::TestDatabase;
use crates_io_worker::BackgroundJob;
use flate2::read::GzDecoder;
use insta::assert_snapshot;
use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use std::io::Read;
use std::sync::Mutex;
use tar::Archive;

/// Mutex to ensure that only one test is dumping the database at a time, since
/// the dump directory is shared between all invocations of the background job.
static DUMP_DIR_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[tokio::test(flavor = "multi_thread")]
async fn test_dump_db_job() {
    let _guard = DUMP_DIR_MUTEX.lock();

    let (app, _, _, token) = TestApp::full().with_token();

    app.db(|conn| {
        CrateBuilder::new("test-crate", token.as_model().user_id).expect_build(conn);

        let database_url = app.as_inner().config.db.primary.url.expose_secret();
        DumpDb::new(database_url).enqueue(conn).unwrap();
    });

    app.run_pending_background_jobs().await;

    let stored_files = app.stored_files().await;
    assert_eq!(stored_files.len(), 1);
    assert_eq!(stored_files[0], "db-dump.tar.gz");

    let path = object_store::path::Path::parse("db-dump.tar.gz").unwrap();
    let result = app.as_inner().storage.as_inner().get(&path).await.unwrap();
    let bytes = result.bytes().await.unwrap();

    let gz = GzDecoder::new(bytes.reader());
    let mut tar = Archive::new(gz);
    assert_snapshot!(replace_dates(&create_tar_snapshot(&mut tar)));
}

/// Create a stringified snapshot of the contents of a tar archive.
///
/// This function reads the contents of the tar archive and returns a string
/// representation of the paths and contents of the files in the archive.
///
/// The contents of the files are assumed to be UTF-8 encoded text. Binary
/// files are not supported.
fn create_tar_snapshot<R: Read>(archive: &mut Archive<R>) -> String {
    const SEPARATOR: &str = "\n----------------------------------------\n";

    let mut contents = Vec::new();

    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().display().to_string();

        let mut content = Vec::new();
        entry.read_to_end(&mut content).unwrap();
        let content = String::from_utf8(content).unwrap();

        contents.push((path, content));
    }

    let paths = contents
        .iter()
        .map(|(path, _)| format!("- {path}\n"))
        .collect::<Vec<_>>()
        .join("");

    let contents = contents
        .iter()
        .map(|(path, content)| format!("{path}:\n\n{content}"))
        .collect::<Vec<_>>()
        .join(SEPARATOR);

    format!("{paths}{SEPARATOR}{contents}")
}

/// Replace dates in a string with a fixed date to make snapshots stable.
///
/// This function replaces dates in the formats:
///
/// - `YYYY-MM-DD-HHMMSS` (e.g. `2024-12-24-123456`)
/// - `YYYY-MM-DD HH:MM:SS.SSS` (e.g. `2024-12-24 12:34:56.789012`)
/// - `YYYY-MM-DDTHH:MM:SS.SSSZ` (e.g. `2024-12-24T12:34:56.789012Z`)
fn replace_dates(s: &str) -> String {
    let path_date_regex = regex::Regex::new(r"\d{4}-\d{2}-\d{2}-\d{6}").unwrap();
    let s = path_date_regex.replace_all(s, "2024-12-24-123456");

    let sql_date_regex = regex::Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}.\d+").unwrap();
    let s = sql_date_regex.replace_all(&s, "2024-12-24 12:34:56.789012");

    let iso_date_regex = regex::Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d+Z").unwrap();
    iso_date_regex
        .replace_all(&s, "2024-12-24T12:34:56.789012Z")
        .to_string()
}

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
