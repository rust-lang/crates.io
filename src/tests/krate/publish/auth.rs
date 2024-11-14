use crate::schema::api_tokens;
use crate::tests::builders::{CrateBuilder, PublishBuilder};
use crate::tests::util::{RequestHelper, TestApp};
use diesel::ExpressionMethods;
use diesel_async::RunQueryDsl;
use googletest::prelude::*;
use http::StatusCode;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn new_wrong_token() {
    let (app, anon, _, token) = TestApp::full().with_token();
    let mut conn = app.async_db_conn().await;

    // Try to publish without a token
    let crate_to_publish = PublishBuilder::new("foo", "1.0.0");
    let response = anon.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

    // Try to publish with the wrong token (by changing the token in the database)
    diesel::update(api_tokens::table)
        .set(api_tokens::token.eq(b"bad" as &[u8]))
        .execute(&mut conn)
        .await
        .unwrap();

    let crate_to_publish = PublishBuilder::new("foo", "1.0.0");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"authentication failed"}]}"#);
    assert_that!(app.stored_files().await, empty());
    assert_that!(app.emails(), empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_wrong_user() {
    let (app, _, user) = TestApp::full().with_user();
    let mut conn = app.db_conn();

    // Create the foo_wrong crate with one user
    CrateBuilder::new("foo_wrong", user.as_model().id).expect_build(&mut conn);

    // Then try to publish with a different user
    let another_user = app.db_new_user("another").db_new_token("bar");
    let crate_to_publish = PublishBuilder::new("foo_wrong", "2.0.0");

    let response = another_user.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this crate exists but you don't seem to be an owner. If you believe this is a mistake, perhaps you need to accept an invitation to be an owner before publishing."}]}"#);

    assert_that!(app.stored_files().await, empty());
    assert_that!(app.emails(), empty());
}
