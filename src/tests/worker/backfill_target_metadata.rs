use crate::builders::CrateBuilder;
use crate::util::TestApp;
use claims::{assert_err, assert_none, assert_ok, assert_some};
use crates_io::schema::{background_jobs, versions};
use crates_io::storage::StorageKey;
use crates_io::worker::jobs::BackfillTargetMetadata;
use crates_io_crate_zip::build_zip;
use crates_io_database::models::VersionTargetMetadata;
use crates_io_tarball::TarballBuilder;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use insta::{assert_json_snapshot, assert_snapshot};
use std::io::Cursor;

const CRATE_NAME: &str = "target-metadata";
const VERSION: &str = "1.0.0";

async fn create_version(conn: &mut AsyncPgConnection, user_id: i32) -> i32 {
    let krate = CrateBuilder::new(CRATE_NAME, user_id)
        .version(VERSION)
        .expect_build(conn)
        .await;

    versions::table
        .filter(versions::crate_id.eq(krate.id))
        .select(versions::id)
        .get_result(conn)
        .await
        .unwrap()
}

async fn upload_source_archive(app: &TestApp, cargo_toml: &[u8], files: &[(&str, &[u8])]) {
    let root = format!("{CRATE_NAME}-{VERSION}");
    let mut tarball = TarballBuilder::new().add_file(&format!("{root}/Cargo.toml"), cargo_toml);
    for (path, contents) in files {
        tarball = tarball.add_file(&format!("{root}/{path}"), contents);
    }

    let mut tarball = Cursor::new(tarball.build());
    let mut zip = Cursor::new(Vec::new());
    let modified = assert_ok!(zip::DateTime::from_date_and_time(2020, 1, 2, 3, 4, 6));
    let manifest = assert_ok!(build_zip(&mut tarball, modified, &mut zip));
    let storage = &app.as_inner().storage;

    let zip_key = StorageKey::for_crate_zip(CRATE_NAME, VERSION);
    assert_ok!(storage.upload(&zip_key, zip.into_inner().into()).await);

    let manifest_key = StorageKey::for_crate_zip_manifest(CRATE_NAME, VERSION);
    let manifest = assert_ok!(serde_json::to_vec(&manifest));
    assert_ok!(storage.upload(&manifest_key, manifest.into()).await);
}

async fn run_job(app: &TestApp, conn: &AsyncPgConnection, version_id: i32) {
    assert_ok!(BackfillTargetMetadata::new(version_id).enqueue(conn).await);
    app.run_pending_background_jobs().await;
}

async fn run_failed_job(
    app: &TestApp,
    conn: &mut AsyncPgConnection,
    version_id: i32,
) -> anyhow::Error {
    BackfillTargetMetadata::new(version_id)
        .enqueue(&*conn)
        .await
        .unwrap();

    let error = assert_err!(app.try_run_pending_background_jobs().await);
    assert_none!(target_metadata(conn, version_id).await);

    diesel::delete(background_jobs::table)
        .execute(conn)
        .await
        .unwrap();

    error
}

async fn target_metadata(
    conn: &mut AsyncPgConnection,
    version_id: i32,
) -> Option<VersionTargetMetadata> {
    versions::table
        .find(version_id)
        .select(versions::target_metadata)
        .get_result(conn)
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn backfills_target_metadata() {
    let (app, _, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;
    let version_id = create_version(&mut conn, user.as_model().id).await;
    let cargo_toml = br#"
        [package]
        name = "target-metadata"
        version = "1.0.0"
        build = "build/custom.rs"

        [lib]
        proc-macro = true

        [[bin]]
        name = "tool"
        path = "src/tool.rs"
    "#;
    upload_source_archive(
        &app,
        cargo_toml,
        &[
            ("build/custom.rs", b"fn main() {}"),
            ("src/lib.rs", b""),
            ("src/tool.rs", b"fn main() {}"),
        ],
    )
    .await;

    run_job(&app, &conn, version_id).await;

    let metadata = assert_some!(target_metadata(&mut conn, version_id).await);
    assert_json_snapshot!(metadata.0, @r#"
    {
      "build_script": {
        "path": "build/custom.rs"
      },
      "library": {
        "path": "src/lib.rs",
        "name": "target_metadata",
        "is_proc_macro": true
      },
      "binaries": [
        {
          "path": "src/tool.rs",
          "name": "tool"
        }
      ]
    }
    "#);
}

#[tokio::test(flavor = "multi_thread")]
async fn preserves_missing_target_sources() {
    let (app, _, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;
    let version_id = create_version(&mut conn, user.as_model().id).await;
    let cargo_toml = br#"
        [package]
        name = "target-metadata"
        version = "1.0.0"
        build = "build.rs"

        [lib]
        path = "src/lib.rs"

        [[bin]]
        name = "tool"
        path = "src/tool.rs"
    "#;
    upload_source_archive(&app, cargo_toml, &[]).await;

    run_job(&app, &conn, version_id).await;

    let metadata = assert_some!(target_metadata(&mut conn, version_id).await);
    assert_json_snapshot!(metadata.0, @r#"
    {
      "build_script": {
        "path": "build.rs",
        "exists": false
      },
      "library": {
        "path": "src/lib.rs",
        "exists": false,
        "name": "target_metadata",
        "is_proc_macro": false
      },
      "binaries": [
        {
          "path": "src/tool.rs",
          "exists": false,
          "name": "tool"
        }
      ]
    }
    "#);
}

#[tokio::test(flavor = "multi_thread")]
async fn leaves_metadata_uncollected_when_the_zip_is_missing() {
    let (app, _, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;
    let version_id = create_version(&mut conn, user.as_model().id).await;
    upload_source_archive(
        &app,
        b"[package]\nname = \"target-metadata\"\nversion = \"1.0.0\"",
        &[],
    )
    .await;
    let zip_key = StorageKey::for_crate_zip(CRATE_NAME, VERSION);
    assert_ok!(app.as_inner().storage.delete(&zip_key).await);

    let error = run_failed_job(&app, &mut conn, version_id).await;
    assert_snapshot!(error, @"1 jobs failed");
}

#[tokio::test(flavor = "multi_thread")]
async fn leaves_metadata_uncollected_when_the_zip_manifest_is_missing() {
    let (app, _, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;
    let version_id = create_version(&mut conn, user.as_model().id).await;

    let error = run_failed_job(&app, &mut conn, version_id).await;
    assert_snapshot!(error, @"1 jobs failed");
}
