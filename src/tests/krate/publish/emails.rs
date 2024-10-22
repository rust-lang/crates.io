use crate::schema::emails;
use crate::tests::builders::PublishBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use diesel::{delete, update, ExpressionMethods};
use diesel_async::RunQueryDsl;
use googletest::prelude::*;

use http::StatusCode;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_without_any_email_fails() {
    let (app, _, _, token) = TestApp::full().with_token();
    let mut conn = app.async_db_conn().await;

    delete(emails::table).execute(&mut conn).await.unwrap();

    let crate_to_publish = PublishBuilder::new("foo_no_email", "1.0.0");

    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"A verified email address is required to publish crates to crates.io. Visit https://crates.io/settings/profile to set and verify your email address."}]}"#);
    assert_that!(app.stored_files().await, empty());
    assert_that!(app.emails(), empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_with_unverified_email_fails() {
    let (app, _, _, token) = TestApp::full().with_token();
    let mut conn = app.async_db_conn().await;

    update(emails::table)
        .set((emails::verified.eq(false),))
        .execute(&mut conn)
        .await
        .unwrap();

    let crate_to_publish = PublishBuilder::new("foo_unverified_email", "1.0.0");

    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"A verified email address is required to publish crates to crates.io. Visit https://crates.io/settings/profile to set and verify your email address."}]}"#);
    assert_that!(app.stored_files().await, empty());
    assert_that!(app.emails(), empty());
}
