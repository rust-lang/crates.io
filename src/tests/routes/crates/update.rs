use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn test_enable_trustpub_only() {
    let (app, _, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    // Create a crate
    let owner_id = user.as_model().id;
    CrateBuilder::new("foo", owner_id)
        .expect_build(&mut conn)
        .await;

    let url = "/api/v1/crates/foo";
    let body = serde_json::json!({ "trustpub_only": true });
    let response = user.patch::<()>(url, body.to_string()).await;
    assert_snapshot!(response.status(), @"200 OK");
    let json = response.json();
    assert_json_snapshot!(json["crate"]["trustpub_only"], @"true");
    assert_json_snapshot!(json, {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    // Verify the flag was set
    let response = user.get::<()>(url).await;
    assert_snapshot!(response.status(), @"200 OK");
    let json = response.json();
    assert_json_snapshot!(json["crate"]["trustpub_only"], @"true");
    assert_json_snapshot!(json, {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });

    assert_snapshot!(app.emails_snapshot().await);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_disable_trustpub_only() {
    let (app, _, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    // Create a crate with trustpub_only enabled
    let owner_id = user.as_model().id;
    CrateBuilder::new("foo", owner_id)
        .trustpub_only(true)
        .expect_build(&mut conn)
        .await;

    let url = "/api/v1/crates/foo";
    let body = serde_json::json!({ "trustpub_only": false });
    let response = user.patch::<()>(url, body.to_string()).await;
    assert_snapshot!(response.status(), @"200 OK");
    let json = response.json();
    assert_json_snapshot!(json["crate"]["trustpub_only"], @"false");
    assert_json_snapshot!(json, {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    // Verify the flag was unset
    let response = user.get::<()>(url).await;
    assert_snapshot!(response.status(), @"200 OK");
    let json = response.json();
    assert_json_snapshot!(json["crate"]["trustpub_only"], @"false");
    assert_json_snapshot!(json, {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });

    assert_snapshot!(app.emails_snapshot().await);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_update_trustpub_only_requires_authentication() {
    let (app, anon, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    // Create a crate
    let owner_id = user.as_model().id;
    CrateBuilder::new("foo", owner_id)
        .expect_build(&mut conn)
        .await;

    // Try to update as an unauthenticated user
    let url = "/api/v1/crates/foo";
    let body = serde_json::json!({ "trustpub_only": true });
    let response = anon.patch::<()>(url, body.to_string()).await;
    assert_snapshot!(response.status(), @"403 Forbidden");

    assert_eq!(app.emails().await.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_update_trustpub_only_requires_ownership() {
    let (app, _, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    // Create a crate with one user
    let owner_id = user.as_model().id;
    CrateBuilder::new("foo", owner_id)
        .expect_build(&mut conn)
        .await;

    // Create a different user
    let another_user = app.db_new_user("another").await;

    // Try to update with a different user
    let url = "/api/v1/crates/foo";
    let body = serde_json::json!({ "trustpub_only": true });
    let response = another_user.patch::<()>(url, body.to_string()).await;
    assert_snapshot!(response.status(), @"403 Forbidden");

    assert_eq!(app.emails().await.len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_update_nonexistent_crate() {
    let (app, _, user) = TestApp::full().with_user().await;

    let url = "/api/v1/crates/nonexistent";
    let body = serde_json::json!({ "trustpub_only": true });
    let response = user.patch::<()>(url, body.to_string()).await;
    assert_snapshot!(response.status(), @"404 Not Found");

    assert_eq!(app.emails().await.len(), 0);
}
