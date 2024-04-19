use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_json_snapshot;

async fn version_with_build_metadata(v1: &str, v2: &str, expected_error: &str) {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token
        .async_publish_crate(PublishBuilder::new("foo", v1))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    let response = token
        .async_publish_crate(PublishBuilder::new("foo", v2))
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": expected_error }] })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn version_with_build_metadata_1() {
    let mut settings = insta::Settings::new();
    settings.set_snapshot_suffix("build_metadata_1");
    settings
        .bind_async(version_with_build_metadata(
            "1.0.0+foo",
            "1.0.0+bar",
            "crate version `1.0.0` is already uploaded",
        ))
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn version_with_build_metadata_2() {
    let mut settings = insta::Settings::new();
    settings.set_snapshot_suffix("build_metadata_2");
    settings
        .bind_async(version_with_build_metadata(
            "1.0.0-beta.1",
            "1.0.0-beta.1+2",
            "crate version `1.0.0-beta.1` is already uploaded",
        ))
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn version_with_build_metadata_3() {
    let mut settings = insta::Settings::new();
    settings.set_snapshot_suffix("build_metadata_3");
    settings
        .bind_async(version_with_build_metadata(
            "1.0.0+foo",
            "1.0.0",
            "crate version `1.0.0` is already uploaded",
        ))
        .await;
}
