use crate::tests::builders::CrateBuilder;
use crate::tests::util::TestApp;
use crate::worker::jobs::DumpDb;
use bytes::Buf;
use crates_io_worker::BackgroundJob;
use flate2::read::GzDecoder;
use insta::{assert_debug_snapshot, assert_snapshot};
use regex::Regex;
use std::io::{Cursor, Read};
use std::sync::LazyLock;
use tar::Archive;

static PATH_DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}-\d{6}").unwrap());

#[tokio::test(flavor = "multi_thread")]
async fn test_dump_db_job() {
    let (app, _, _, token) = TestApp::full().with_token();
    let mut conn = app.db_conn();

    CrateBuilder::new("test-crate", token.as_model().user_id).expect_build(&mut conn);

    DumpDb.enqueue(&mut conn).unwrap();

    app.run_pending_background_jobs().await;

    assert_snapshot!(app.stored_files().await.join("\n"), @r"
    db-dump.tar.gz
    db-dump.zip
    ");

    let path = object_store::path::Path::parse("db-dump.tar.gz").unwrap();
    let result = app.as_inner().storage.as_inner().get(&path).await.unwrap();
    let bytes = result.bytes().await.unwrap();

    let gz = GzDecoder::new(bytes.reader());
    let mut tar = Archive::new(gz);

    let paths = tar_paths(&mut tar);
    assert_debug_snapshot!(paths, @r#"
    [
        "YYYY-MM-DD-HHMMSS",
        "YYYY-MM-DD-HHMMSS/README.md",
        "YYYY-MM-DD-HHMMSS/export.sql",
        "YYYY-MM-DD-HHMMSS/import.sql",
        "YYYY-MM-DD-HHMMSS/metadata.json",
        "YYYY-MM-DD-HHMMSS/schema.sql",
        "YYYY-MM-DD-HHMMSS/data",
        "YYYY-MM-DD-HHMMSS/data/categories.csv",
        "YYYY-MM-DD-HHMMSS/data/crate_downloads.csv",
        "YYYY-MM-DD-HHMMSS/data/crates.csv",
        "YYYY-MM-DD-HHMMSS/data/keywords.csv",
        "YYYY-MM-DD-HHMMSS/data/metadata.csv",
        "YYYY-MM-DD-HHMMSS/data/reserved_crate_names.csv",
        "YYYY-MM-DD-HHMMSS/data/teams.csv",
        "YYYY-MM-DD-HHMMSS/data/users.csv",
        "YYYY-MM-DD-HHMMSS/data/crates_categories.csv",
        "YYYY-MM-DD-HHMMSS/data/crates_keywords.csv",
        "YYYY-MM-DD-HHMMSS/data/crate_owners.csv",
        "YYYY-MM-DD-HHMMSS/data/versions.csv",
        "YYYY-MM-DD-HHMMSS/data/default_versions.csv",
        "YYYY-MM-DD-HHMMSS/data/dependencies.csv",
        "YYYY-MM-DD-HHMMSS/data/version_downloads.csv",
    ]
    "#);

    let path = object_store::path::Path::parse("db-dump.zip").unwrap();
    let result = app.as_inner().storage.as_inner().get(&path).await.unwrap();
    let bytes = result.bytes().await.unwrap();

    let archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let zip_paths = archive.file_names().collect::<Vec<_>>();
    assert_debug_snapshot!(zip_paths, @r#"
    [
        "README.md",
        "export.sql",
        "import.sql",
        "metadata.json",
        "schema.sql",
        "data/",
        "data/categories.csv",
        "data/crate_downloads.csv",
        "data/crates.csv",
        "data/keywords.csv",
        "data/metadata.csv",
        "data/reserved_crate_names.csv",
        "data/teams.csv",
        "data/users.csv",
        "data/crates_categories.csv",
        "data/crates_keywords.csv",
        "data/crate_owners.csv",
        "data/versions.csv",
        "data/default_versions.csv",
        "data/dependencies.csv",
        "data/version_downloads.csv",
    ]
    "#);
}

fn tar_paths<R: Read>(archive: &mut Archive<R>) -> Vec<String> {
    archive
        .entries()
        .unwrap()
        .map(|entry| entry.unwrap().path().unwrap().display().to_string())
        .map(|path| PATH_DATE_RE.replace(&path, "YYYY-MM-DD-HHMMSS").to_string())
        .collect()
}
