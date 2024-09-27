use crate::models::Crate;
use crate::tests::builders::PublishBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use crate::worker::jobs;
use diesel::prelude::*;
use http::StatusCode;

#[tokio::test(flavor = "multi_thread")]
async fn index_smoke_test() {
    let (app, _, _, token) = TestApp::full().with_token();
    let upstream = app.upstream_index();

    // Add a new crate

    let body = PublishBuilder::new("serde", "1.0.0").body();
    let response = token.put::<()>("/api/v1/crates/new", body).await;
    assert_eq!(response.status(), StatusCode::OK);

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
    assert_eq!(response.status(), StatusCode::OK);

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

    app.db(|conn| {
        use crate::schema::crates;

        let krate: Crate = assert_ok!(Crate::by_name("serde").first(conn));
        assert_ok!(diesel::delete(crates::table.find(krate.id)).execute(conn));

        assert_ok!(jobs::enqueue_sync_to_index("serde", conn));
    });

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

    let (app, _, _, token) = TestApp::full().with_token();
    let upstream = app.upstream_index();

    // Initialize upstream index with a `config.json` file
    upstream.write_file("config.json", ORIGINAL_CONFIG).unwrap();
    assert_ok_eq!(upstream.read_file("config.json"), ORIGINAL_CONFIG);

    // Add a new crate
    let body = PublishBuilder::new("serde", "1.0.0").body();
    let response = token.publish_crate(body).await;
    assert_eq!(response.status(), StatusCode::OK);

    // Adjust the `config.json` file on the upstream index
    upstream.write_file("config.json", UPDATED_CONFIG).unwrap();
    assert_ok_eq!(upstream.read_file("config.json"), UPDATED_CONFIG);

    // Update the crate
    let body = PublishBuilder::new("serde", "1.1.0").body();
    let response = token.publish_crate(body).await;
    assert_eq!(response.status(), StatusCode::OK);

    // Check that the `config.json` changes on the upstream index are preserved
    assert_ok_eq!(upstream.read_file("config.json"), UPDATED_CONFIG);
}
