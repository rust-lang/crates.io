use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use crates_io::schema::emails;
use diesel::{delete, update, ExpressionMethods, RunQueryDsl};
use googletest::prelude::*;
use http::StatusCode;
use insta::assert_json_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_without_any_email_fails() {
    let (app, _, _, token) = TestApp::full().with_token();

    app.db(|conn| {
        delete(emails::table).execute(conn).unwrap();
    });

    let crate_to_publish = PublishBuilder::new("foo_no_email", "1.0.0");

    let response = token.async_publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
    assert_that!(app.async_stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_with_unverified_email_fails() {
    let (app, _, _, token) = TestApp::full().with_token();

    app.db(|conn| {
        update(emails::table)
            .set((emails::verified.eq(false),))
            .execute(conn)
            .unwrap();
    });

    let crate_to_publish = PublishBuilder::new("foo_unverified_email", "1.0.0");

    let response = token.async_publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
    assert_that!(app.async_stored_files().await, empty());
}
