use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use crates_io::schema::emails;
use diesel::{ExpressionMethods, delete, update};
use diesel_async::RunQueryDsl;
use googletest::prelude::*;

use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_without_any_email_fails() {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    delete(emails::table).execute(&mut conn).await.unwrap();

    let crate_to_publish = PublishBuilder::new("foo_no_email", "1.0.0");

    let response = token.publish_crate(crate_to_publish).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"A verified email address is required to publish crates to crates.io. Visit https://crates.io/settings/profile to set and verify your email address."}]}"#);
    assert_that!(app.stored_files().await, is_empty());
    assert_that!(app.emails().await, is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_with_unverified_email_fails() {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    update(emails::table)
        .set((emails::verified.eq(false),))
        .execute(&mut conn)
        .await
        .unwrap();

    let crate_to_publish = PublishBuilder::new("foo_unverified_email", "1.0.0");

    let response = token.publish_crate(crate_to_publish).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"A verified email address is required to publish crates to crates.io. Visit https://crates.io/settings/profile to set and verify your email address."}]}"#);
    assert_that!(app.stored_files().await, is_empty());
    assert_that!(app.emails().await, is_empty());
}
