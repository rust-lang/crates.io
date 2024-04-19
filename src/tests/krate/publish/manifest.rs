use crate::builders::PublishBuilder;
use crate::util::insta::{any_id_redaction, id_redaction};
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_json_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn boolean_readme() {
    // see https://github.com/rust-lang/crates.io/issues/6847

    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token
        .publish_crate(PublishBuilder::new("foo", "1.0.0").custom_manifest(
            r#"[package]
            name = "foo"
            version = "1.0.0"
            description = "description"
            license = "MIT"
            rust-version = "1.69"
            readme = false"#,
        ))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    let response = token.get::<()>("/api/v1/crates/foo/1.0.0").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".version.id" => any_id_redaction(),
        ".version.created_at" => "[datetime]",
        ".version.updated_at" => "[datetime]",
        ".version.published_by.id" => id_redaction(token.as_model().user_id),
        ".version.audit_actions[].time" => "[datetime]",
        ".version.audit_actions[].user.id" => id_redaction(token.as_model().user_id),
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_manifest() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token
        .publish_crate(PublishBuilder::new("foo", "1.0.0").no_manifest())
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
}

#[tokio::test(flavor = "multi_thread")]
async fn manifest_casing() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token
        .publish_crate(
            PublishBuilder::new("foo", "1.0.0")
                .add_file(
                    "foo-1.0.0/CARGO.TOML",
                    "[package]\nname = \"foo\"\nversion = \"1.0.0\"\n",
                )
                .no_manifest(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
}

#[tokio::test(flavor = "multi_thread")]
async fn multiple_manifests() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token
        .publish_crate(
            PublishBuilder::new("foo", "1.0.0")
                .add_file(
                    "foo-1.0.0/Cargo.toml",
                    "[package]\nname = \"foo\"\nversion = \"1.0.0\"\n",
                )
                .add_file(
                    "foo-1.0.0/cargo.toml",
                    "[package]\nname = \"foo\"\nversion = \"1.0.0\"\n",
                )
                .no_manifest(),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_manifest() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token
        .publish_crate(PublishBuilder::new("foo", "1.0.0").custom_manifest(""))
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_manifest_missing_name() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token
        .publish_crate(
            PublishBuilder::new("foo", "1.0.0").custom_manifest("[package]\nversion = \"1.0.0\""),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_manifest_missing_version() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token
        .publish_crate(
            PublishBuilder::new("foo", "1.0.0").custom_manifest("[package]\nname = \"foo\""),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_rust_version() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response =
        token.publish_crate(PublishBuilder::new("foo", "1.0.0").custom_manifest(
            "[package]\nname = \"foo\"\nversion = \"1.0.0\"\ndescription = \"description\"\nlicense = \"MIT\"\nrust-version = \"\"\n",
        )).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());

    let response = token.publish_crate(PublishBuilder::new("foo", "1.0.0").custom_manifest(
        "[package]\nname = \"foo\"\nversion = \"1.0.0\"\ndescription = \"description\"\nlicense = \"MIT\"\nrust-version = \"1.0.0-beta.2\"\n",
    )).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
}
