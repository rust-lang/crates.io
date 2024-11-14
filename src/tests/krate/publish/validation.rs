use crate::models::krate::MAX_NAME_LENGTH;
use crate::tests::builders::PublishBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use googletest::prelude::*;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn empty_json() {
    let (app, _, _, token) = TestApp::full().with_token();

    let (_json, tarball) = PublishBuilder::new("foo", "1.0.0").build();
    let body = PublishBuilder::create_publish_body("{}", &tarball);

    let response = token.publish_crate(body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid upload request: missing field `name` at line 1 column 2"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_names() {
    let (app, _, _, token) = TestApp::full().with_token();

    async fn bad_name(name: &str, client: &impl RequestHelper) {
        let crate_to_publish = PublishBuilder::new(name, "1.0.0");
        let response = client.publish_crate(crate_to_publish).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_json_snapshot!(response.json());
    }

    bad_name("", &token).await;
    bad_name("foo bar", &token).await;
    bad_name(&"a".repeat(MAX_NAME_LENGTH + 1), &token).await;
    bad_name("snow☃", &token).await;
    bad_name("áccênts", &token).await;

    bad_name("std", &token).await;
    bad_name("STD", &token).await;
    bad_name("compiler-rt", &token).await;
    bad_name("compiler_rt", &token).await;
    bad_name("coMpiLer_Rt", &token).await;

    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_version() {
    let (app, _, _, token) = TestApp::full().with_token();

    let (json, tarball) = PublishBuilder::new("foo", "1.0.0").build();
    let new_json = json.replace(r#""vers":"1.0.0""#, r#""vers":"broken""#);
    assert_ne!(json, new_json);
    let body = PublishBuilder::create_publish_body(&new_json, &tarball);

    let response = token.publish_crate(body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"\"broken\" is an invalid semver version"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn license_and_description_required() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_metadata", "1.1.0")
        .unset_license()
        .unset_description();

    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"missing or empty metadata fields: description, license. Please see https://doc.rust-lang.org/cargo/reference/manifest.html for more information on configuring these fields"}]}"#);
    let crate_to_publish = PublishBuilder::new("foo_metadata", "1.1.0").unset_description();

    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"missing or empty metadata fields: description. Please see https://doc.rust-lang.org/cargo/reference/manifest.html for more information on configuring these fields"}]}"#);
    let crate_to_publish = PublishBuilder::new("foo_metadata", "1.1.0")
        .unset_license()
        .license_file("foo")
        .unset_description();

    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"missing or empty metadata fields: description. Please see https://doc.rust-lang.org/cargo/reference/manifest.html for more information on configuring these fields"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn long_description() {
    let (app, _, _, token) = TestApp::full().with_token();

    let description = "a".repeat(2000);
    let crate_to_publish = PublishBuilder::new("foo_metadata", "1.1.0").description(&description);

    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"The `description` is too long. A maximum of 1000 characters are currently allowed."}]}"#);

    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_license() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token
        .publish_crate(PublishBuilder::new("foo", "1.0.0").license("MIT AND foobar"))
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"unknown or invalid license expression; see http://opensource.org/licenses for options, and http://spdx.org/licenses/ for their identifiers\nNote: If you have a non-standard license that is not listed by SPDX, use the license-file field to specify the path to a file containing the text of the license.\nSee https://doc.rust-lang.org/cargo/reference/manifest.html#the-license-and-license-file-fields for more information.\nMIT AND foobar\n        ^^^^^^ unknown term"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_urls() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token
        .publish_crate(
            PublishBuilder::new("foo", "1.0.0").documentation("javascript:alert('boom')"),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"URL for field `documentation` must begin with http:// or https:// (url: javascript:alert('boom'))"}]}"#);
    assert_that!(app.stored_files().await, empty());
}
