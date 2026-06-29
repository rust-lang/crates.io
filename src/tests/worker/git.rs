use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use claims::{assert_ok, assert_ok_eq};
use crates_io::models::Crate;
use crates_io::worker::jobs;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn index_smoke_test() {
    let (app, _, _, token) = TestApp::full().with_git_index().with_token().await;
    let mut conn = app.db_conn().await;
    let upstream = app.upstream_index();

    // Add a new crate

    let body = PublishBuilder::new("serde", "1.0.0").body();
    let response = token.put::<()>("/api/v1/crates/new", body).await;
    assert_snapshot!(response.status(), @"200 OK");

    // Check that the git index is updated asynchronously
    assert_ok_eq!(upstream.list_commits(), vec!["Initial Commit"]);
    assert_ok_eq!(upstream.crate_exists("serde"), false);

    app.run_pending_background_jobs().await;
    assert_ok_eq!(
        upstream.list_commits(),
        vec!["Initial Commit", "Create crate `serde`"]
    );
    assert_ok_eq!(upstream.crate_exists("serde"), true);

    // Yank the crate

    let response = token.delete::<()>("/api/v1/crates/serde/1.0.0/yank").await;
    assert_snapshot!(response.status(), @"200 OK");

    app.run_pending_background_jobs().await;
    assert_ok_eq!(
        upstream.list_commits(),
        vec![
            "Initial Commit",
            "Create crate `serde`",
            "Update crate `serde`",
        ]
    );
    assert_ok_eq!(upstream.crate_exists("serde"), true);

    // Delete the crate

    use crates_io::schema::crates;

    let krate: Crate = assert_ok!(Crate::by_name("serde").first(&mut conn).await);
    assert_ok!(
        diesel::delete(crates::table.find(krate.id))
            .execute(&mut conn)
            .await
    );

    assert_ok!(jobs::SyncToGitIndex::new("serde").enqueue(&conn).await);
    assert_ok!(jobs::SyncToSparseIndex::new("serde").enqueue(&conn).await);

    app.run_pending_background_jobs().await;
    assert_ok_eq!(
        upstream.list_commits(),
        vec![
            "Initial Commit",
            "Create crate `serde`",
            "Update crate `serde`",
            "Delete crate `serde`",
        ]
    );
    assert_ok_eq!(upstream.crate_exists("serde"), false);
}

/// This test checks that changes to the `config.json` file on the git index
/// are preserved when the background worker updates the index.
#[tokio::test(flavor = "multi_thread")]
async fn test_config_changes() {
    const ORIGINAL_CONFIG: &str = r#"{
        "dl": "https://crates.io/api/v1/crates",
        "api": "https://crates.io"
    }"#;

    const UPDATED_CONFIG: &str = r#"{
        "dl": "https://static.crates.io/crates",
        "api": "https://crates.io"
    }"#;

    let (app, _, _, token) = TestApp::full().with_git_index().with_token().await;
    let upstream = app.upstream_index();

    // Initialize upstream index with a `config.json` file
    upstream.write_file("config.json", ORIGINAL_CONFIG).unwrap();
    assert_ok_eq!(upstream.read_file("config.json"), ORIGINAL_CONFIG);

    // Add a new crate
    let body = PublishBuilder::new("serde", "1.0.0").body();
    let response = token.publish_crate(body).await;
    assert_snapshot!(response.status(), @"200 OK");

    // Adjust the `config.json` file on the upstream index
    upstream.write_file("config.json", UPDATED_CONFIG).unwrap();
    assert_ok_eq!(upstream.read_file("config.json"), UPDATED_CONFIG);

    // Update the crate
    let body = PublishBuilder::new("serde", "1.1.0").body();
    let response = token.publish_crate(body).await;
    assert_snapshot!(response.status(), @"200 OK");

    // Check that the `config.json` changes on the upstream index are preserved
    assert_ok_eq!(upstream.read_file("config.json"), UPDATED_CONFIG);
}

/// Exercises `BulkSyncToGitIndex` with a mix of create/update/delete/no-op
/// crates and verifies that the job produces a single commit covering them
/// all.
#[tokio::test(flavor = "multi_thread")]
async fn bulk_sync_to_git_index() {
    use crates_io::schema::crates;

    let (app, _, _, token) = TestApp::full().with_git_index().with_token().await;
    let mut conn = app.db_conn().await;
    let upstream = app.upstream_index();

    // Publish three crates and let the per-crate sync jobs land them in the
    // upstream index.
    for name in ["alpha", "beta", "gamma"] {
        let body = PublishBuilder::new(name, "1.0.0").body();
        let response = token.put::<()>("/api/v1/crates/new", body).await;
        insta::allow_duplicates! {
            assert_snapshot!(response.status(), @"200 OK");
        }
    }
    app.run_pending_background_jobs().await;
    for name in ["alpha", "beta", "gamma"] {
        assert_ok_eq!(upstream.crate_exists(name), true);
    }

    // Capture the DB-derived contents that `SyncToGitIndex` wrote above so
    // we can assert the bulk sync restores them after corruption.
    let beta_expected = upstream.read_file("be/ta/beta").unwrap();
    let gamma_expected = upstream.read_file("ga/mm/gamma").unwrap();

    // Delete `alpha` from the DB so the bulk sync sees it as "present in
    // index, missing from DB" → delete. Corrupt `beta`'s entry so the bulk
    // sync sees it as stale → update. Leave `gamma` untouched → no-op.
    let alpha: Crate = assert_ok!(Crate::by_name("alpha").first(&mut conn).await);
    assert_ok!(
        diesel::delete(crates::table.find(alpha.id))
            .execute(&mut conn)
            .await
    );
    upstream.write_file("be/ta/beta", "stale\n").unwrap();

    let before = upstream.list_commits().unwrap().len();

    // Enqueue a bulk sync covering all three names.
    let bulk = jobs::BulkSyncToGitIndex::new(
        vec!["alpha".into(), "beta".into(), "gamma".into()],
        "Bulk sync",
    );
    assert_ok!(bulk.enqueue(&conn).await);
    app.run_pending_background_jobs().await;

    // Exactly one new commit on `master`, touching alpha (delete) + beta
    // (update) in a single commit.
    let commits = upstream.list_commits().unwrap();
    assert_eq!(commits.len(), before + 1);
    assert_eq!(commits.last().unwrap(), "Bulk sync");

    // alpha's entry was deleted.
    assert_ok_eq!(upstream.crate_exists("alpha"), false);
    // beta's entry was restored to the DB-derived bytes.
    assert_eq!(upstream.read_file("be/ta/beta").unwrap(), beta_expected);
    // gamma's entry is unchanged.
    assert_eq!(upstream.read_file("ga/mm/gamma").unwrap(), gamma_expected);
}

/// Exercises the no-op path: a `BulkSyncToGitIndex` over crates whose index
/// entries already match the DB should not create a new commit.
#[tokio::test(flavor = "multi_thread")]
async fn bulk_sync_to_git_index_noop() {
    let (app, _, _, token) = TestApp::full().with_git_index().with_token().await;
    let conn = app.db_conn().await;
    let upstream = app.upstream_index();

    let body = PublishBuilder::new("serde", "1.0.0").body();
    let response = token.put::<()>("/api/v1/crates/new", body).await;
    assert_snapshot!(response.status(), @"200 OK");
    app.run_pending_background_jobs().await;

    let before = upstream.list_commits().unwrap();

    let bulk = jobs::BulkSyncToGitIndex::new(vec!["serde".into()], "no-op bulk");
    assert_ok!(bulk.enqueue(&conn).await);
    app.run_pending_background_jobs().await;

    assert_eq!(upstream.list_commits().unwrap(), before);
}
