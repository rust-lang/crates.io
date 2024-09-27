use crate::tests::builders::PublishBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use googletest::prelude::*;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn good_keywords() {
    let (_, _, _, token) = TestApp::full().with_token();
    let crate_to_publish = PublishBuilder::new("foo_good_key", "1.0.0")
        .keyword("c++")
        .keyword("crates-io_index")
        .keyword("1password");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn bad_keywords() {
    let (_, _, _, token) = TestApp::full().with_token();
    let crate_to_publish =
        PublishBuilder::new("foo_bad_key", "1.0.0").keyword("super-long-keyword-name-oh-no");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"\"super-long-keyword-name-oh-no\" is an invalid keyword (keywords must have less than 20 characters)"}]}"#);

    let crate_to_publish = PublishBuilder::new("foo_bad_key", "1.0.0").keyword("?@?%");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"\"?@?%\" is an invalid keyword"}]}"#);

    let crate_to_publish = PublishBuilder::new("foo_bad_key", "1.0.0").keyword("áccênts");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"\"áccênts\" is an invalid keyword"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn too_many_keywords() {
    let (app, _, _, token) = TestApp::full().with_token();
    let response = token
        .publish_crate(
            PublishBuilder::new("foo", "1.0.0")
                .keyword("one")
                .keyword("two")
                .keyword("three")
                .keyword("four")
                .keyword("five")
                .keyword("six"),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"expected at most 5 keywords per crate"}]}"#);
    assert_that!(app.stored_files().await, empty());
}
