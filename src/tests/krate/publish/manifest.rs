use crate::tests::builders::PublishBuilder;
use crate::tests::util::insta::{any_id_redaction, id_redaction};
use crate::tests::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn boolean_readme() {
    // see https://github.com/rust-lang/crates.io/issues/6847

    let (_app, _anon, _cookie, token) = TestApp::full().with_token().await;

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
    let (_app, _anon, _cookie, token) = TestApp::full().with_token().await;

    let response = token
        .publish_crate(PublishBuilder::new("foo", "1.0.0").no_manifest())
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"uploaded tarball is missing a `Cargo.toml` manifest file"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn manifest_casing() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token().await;

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
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"uploaded tarball is missing a `Cargo.toml` manifest file; `CARGO.TOML` was found, but must be named `Cargo.toml` with that exact casing"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn multiple_manifests() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token().await;

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
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"uploaded tarball contains more than one `Cargo.toml` manifest file; found `Cargo.toml`, `cargo.toml`"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_manifest() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token().await;

    let response = token
        .publish_crate(PublishBuilder::new("foo", "1.0.0").custom_manifest(""))
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"failed to parse `Cargo.toml` manifest file\n\nmissing field `name`\n"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_manifest_missing_name() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token().await;

    let response = token
        .publish_crate(
            PublishBuilder::new("foo", "1.0.0").custom_manifest("[package]\nversion = \"1.0.0\""),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"failed to parse `Cargo.toml` manifest file\n\nTOML parse error at line 1, column 1\n  |\n1 | [package]\n  | ^^^^^^^^^\nmissing field `name`\n"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_manifest_missing_version() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token().await;

    let response = token
        .publish_crate(
            PublishBuilder::new("foo", "1.0.0").custom_manifest("[package]\nname = \"foo\""),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"failed to parse `Cargo.toml` manifest file\n\nmissing field `version`"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_rust_version() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token().await;

    let response =
        token.publish_crate(PublishBuilder::new("foo", "1.0.0").custom_manifest(
            "[package]\nname = \"foo\"\nversion = \"1.0.0\"\ndescription = \"description\"\nlicense = \"MIT\"\nrust-version = \"\"\n",
        )).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"failed to parse `Cargo.toml` manifest file\n\ninvalid `rust-version` value"}]}"#);

    let response = token.publish_crate(PublishBuilder::new("foo", "1.0.0").custom_manifest(
        "[package]\nname = \"foo\"\nversion = \"1.0.0\"\ndescription = \"description\"\nlicense = \"MIT\"\nrust-version = \"1.0.0-beta.2\"\n",
    )).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"failed to parse `Cargo.toml` manifest file\n\ninvalid `rust-version` value"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_lib_and_bin_crate() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token().await;

    let publish_builder = PublishBuilder::new("foo", "1.0.0")
        .add_file("foo-1.0.0/src/lib.rs", "pub fn foo() {}")
        .add_file("foo-1.0.0/src/main.rs", "fn main() {}")
        .add_file("foo-1.0.0/src/bin/bar.rs", "fn main() {}");

    let response = token.publish_crate(publish_builder).await;
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
