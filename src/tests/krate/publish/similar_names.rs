use crate::builders::{CrateBuilder, PublishBuilder};
use crate::util::{RequestHelper, TestApp};
use googletest::prelude::*;
use http::StatusCode;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn new_crate_similar_name() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        CrateBuilder::new("Foo_similar", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo_similar", "1.1.0");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate was previously named `Foo_similar`"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_crate_similar_name_hyphen() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo_bar_hyphen", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo-bar-hyphen", "1.1.0");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate was previously named `foo_bar_hyphen`"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_crate_similar_name_underscore() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo-bar-underscore", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo_bar_underscore", "1.1.0");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate was previously named `foo-bar-underscore`"}]}"#);
    assert_that!(app.stored_files().await, empty());
}
