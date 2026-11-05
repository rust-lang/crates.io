use crate::builders::{CrateBuilder, PublishBuilder};
use crate::util::{RequestHelper, TestApp};
use crates_io::models::token::{CrateScope, EndpointScope};
use googletest::prelude::*;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn new_crate_similar_name() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("Foo_similar", user.as_model().id)
        .version("1.0.0")
        .expect_build(&mut conn)
        .await;

    let crate_to_publish = PublishBuilder::new("foo_similar", "1.1.0");
    let response = token.publish_crate(crate_to_publish).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate was previously named `Foo_similar`"}]}"#);
    assert_that!(app.stored_files().await, is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_crate_similar_name_hyphen() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo_bar_hyphen", user.as_model().id)
        .version("1.0.0")
        .expect_build(&mut conn)
        .await;

    let crate_to_publish = PublishBuilder::new("foo-bar-hyphen", "1.1.0");
    let response = token.publish_crate(crate_to_publish).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate was previously named `foo_bar_hyphen`"}]}"#);
    assert_that!(app.stored_files().await, is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_crate_similar_name_underscore() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo-bar-underscore", user.as_model().id)
        .version("1.0.0")
        .expect_build(&mut conn)
        .await;

    let crate_to_publish = PublishBuilder::new("foo_bar_underscore", "1.1.0");
    let response = token.publish_crate(crate_to_publish).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate was previously named `foo-bar-underscore`"}]}"#);
    assert_that!(app.stored_files().await, is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_crate_similar_name_with_alternate_spelling_crate_scope() {
    let crate_scopes = Some(vec![CrateScope::try_from("foo_*").unwrap()]);
    let endpoint_scopes = Some(vec![EndpointScope::PublishUpdate]);
    let (app, _, _, token) = TestApp::full()
        .with_scoped_token(crate_scopes, endpoint_scopes)
        .await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo-bar", token.as_model().user_id)
        .version("1.0.0")
        .expect_build(&mut conn)
        .await;

    let crate_to_publish = PublishBuilder::new("foo_bar", "1.1.0");
    let response = token.publish_crate(crate_to_publish).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate was previously named `foo-bar`"}]}"#);
    assert_that!(app.stored_files().await, is_empty());
}
