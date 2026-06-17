use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use chrono::TimeZone;
use claims::{assert_ok, assert_some};
use crates_io::schema::versions;
use crates_io::worker::jobs::BuildCrateZip;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use insta::{assert_json_snapshot, assert_snapshot};
use object_store::ObjectStoreExt;

#[tokio::test(flavor = "multi_thread")]
async fn test_build_crate_zip_job() {
    let created_at = chrono::Utc.with_ymd_and_hms(2026, 6, 16, 12, 34, 56);

    let (app, _, _, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    let pb = PublishBuilder::new("zip_test", "1.0.0")
        .add_file("zip_test-1.0.0/src/lib.rs", "fn foo() {}")
        .add_file("zip_test-1.0.0/src/empty.rs", "");

    let response = token.publish_crate(pb).await;
    assert_snapshot!(response.status(), @"200 OK");

    let version_id: i32 = diesel::update(versions::table)
        .set(versions::created_at.eq(created_at.unwrap()))
        .returning(versions::id)
        .get_result(&mut conn)
        .await
        .unwrap();

    BuildCrateZip::new(version_id).enqueue(&conn).await.unwrap();
    app.run_pending_background_jobs().await;

    let (zip_sha256, zip_json_sha256): (Option<Vec<u8>>, Option<Vec<u8>>) = versions::table
        .find(version_id)
        .select((versions::zip_sha256, versions::zip_json_sha256))
        .first(&mut conn)
        .await
        .unwrap();

    let zip_sha256 = hex::encode(assert_some!(zip_sha256));
    assert_snapshot!(zip_sha256, @"503a462ac663487a6b00fb86df197a4ad212683253e907bb4bb82970b30a2977");

    let zip_json_sha256 = hex::encode(assert_some!(zip_json_sha256));
    assert_snapshot!(zip_json_sha256, @"b7c006fd4f04783852562853b8c474226db80eda3ce99dfba5a1815f2abf410f");

    assert_snapshot!(app.stored_files().await.join("\n"), @"
    crates/zip_test/zip_test-1.0.0.crate
    crates/zip_test/zip_test-1.0.0.zip
    crates/zip_test/zip_test-1.0.0.zip.json
    index/zi/p_/zip_test
    rss/crates.xml
    rss/crates/zip_test.xml
    rss/updates.xml
    ");

    let storage = app.as_inner().storage.as_inner();

    let manifest_path = "crates/zip_test/zip_test-1.0.0.zip.json";
    let manifest = assert_ok!(storage.get(&manifest_path.into()).await);

    let manifest = assert_ok!(manifest.bytes().await);
    let manifest: serde_json::Value = assert_ok!(serde_json::from_slice(&manifest));
    assert_json_snapshot!(manifest);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_build_crate_zip_job_nonexistent_version() {
    let (app, _) = TestApp::full().empty().await;
    let conn = app.db_conn().await;

    BuildCrateZip::new(42).enqueue(&conn).await.unwrap();
    app.run_pending_background_jobs().await;

    assert_eq!(app.stored_files().await.len(), 0);
}
