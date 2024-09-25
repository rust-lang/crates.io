use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn workspace_inheritance() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token
        .publish_crate(
            PublishBuilder::new("foo", "1.0.0")
                .custom_manifest("[package]\nname = \"foo\"\nversion.workspace = true\n"),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"failed to parse `Cargo.toml` manifest file\n\nvalue from workspace hasn't been set"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn workspace_inheritance_with_dep() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token.publish_crate(PublishBuilder::new("foo", "1.0.0").custom_manifest(
        "[package]\nname = \"foo\"\nversion = \"1.0.0\"\n\n[dependencies]\nserde.workspace = true\n",
    )).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"failed to parse `Cargo.toml` manifest file\n\nvalue from workspace hasn't been set"}]}"#);
}
