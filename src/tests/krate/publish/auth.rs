use crate::schema::api_tokens;
use crate::tests::builders::{CrateBuilder, PublishBuilder};
use crate::tests::util::{MockTokenUser, RequestHelper, TestApp};
use diesel::ExpressionMethods;
use diesel_async::RunQueryDsl;
use googletest::prelude::*;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn new_wrong_token() {
    let (app, anon, _, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    // Try to publish without a token
    let crate_to_publish = PublishBuilder::new("foo", "1.0.0");
    let response = anon.publish_crate(crate_to_publish).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

    // Try to publish with the wrong token (by changing the token in the database)
    diesel::update(api_tokens::table)
        .set(api_tokens::token.eq(b"bad" as &[u8]))
        .execute(&mut conn)
        .await
        .unwrap();

    let crate_to_publish = PublishBuilder::new("foo", "1.0.0");
    let response = token.publish_crate(crate_to_publish).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"authentication failed"}]}"#);
    assert_that!(app.stored_files().await, is_empty());
    assert_that!(app.emails().await, is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_wrong_user() {
    let (app, _, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    // Create the foo_wrong crate with one user
    CrateBuilder::new("foo_wrong", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    // Then try to publish with a different user
    let another_user = app.db_new_user("another").await;
    let another_user = another_user.db_new_token("bar").await;
    let crate_to_publish = PublishBuilder::new("foo_wrong", "2.0.0");

    let response = another_user.publish_crate(crate_to_publish).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this crate exists but you don't seem to be an owner. If you believe this is a mistake, perhaps you need to accept an invitation to be an owner before publishing."}]}"#);

    assert_that!(app.stored_files().await, is_empty());
    assert_that!(app.emails().await, is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_with_bearer_token() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let token = format!("Bearer {}", token.plaintext());
    let token = MockTokenUser::with_auth_header(token, app.clone());

    let crate_to_publish = PublishBuilder::new("foo_new", "1.0.0");
    let response = token.publish_crate(crate_to_publish).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    assert_snapshot!(app.stored_files().await.join("\n"), @r"
    crates/foo_new/foo_new-1.0.0.crate
    index/fo/o_/foo_new
    rss/crates.xml
    rss/crates/foo_new.xml
    rss/updates.xml
    ");
}
