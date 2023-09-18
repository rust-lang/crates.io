use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_json_snapshot;

#[test]
fn boolean_readme() {
    // see https://github.com/rust-lang/crates.io/issues/6847

    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token.publish_crate(PublishBuilder::new("foo", "1.0.0").custom_manifest(
        r#"[package]
            name = "foo"
            version = "1.0.0"
            rust-version = "1.69"
            readme = false"#,
    ));
    assert_eq!(response.status(), StatusCode::OK);

    let response = token.get::<()>("/api/v1/crates/foo/1.0.0");
    assert_eq!(response.status(), StatusCode::OK);
    let json = response.into_json();
    assert_some_eq!(json["version"]["rust_version"].as_str(), "1.69");
}

#[test]
fn missing_manifest() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token.publish_crate(PublishBuilder::new("foo", "1.0.0").no_manifest());
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "uploaded tarball is missing a `Cargo.toml` manifest file" }] })
    );
}

#[test]
fn manifest_casing() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token.publish_crate(
        PublishBuilder::new("foo", "1.0.0")
            .add_file(
                "foo-1.0.0/CARGO.TOML",
                "[package]\nname = \"foo\"\nversion = \"1.0.0\"\n",
            )
            .no_manifest(),
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.into_json());
}

#[test]
fn multiple_manifests() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token.publish_crate(
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
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.into_json());
}

#[test]
fn invalid_manifest() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token.publish_crate(PublishBuilder::new("foo", "1.0.0").custom_manifest(""));
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "failed to parse `Cargo.toml` manifest file\n\nmissing field `name`\n" }] })
    );
}

#[test]
fn invalid_manifest_missing_name() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token.publish_crate(
        PublishBuilder::new("foo", "1.0.0").custom_manifest("[package]\nversion = \"1.0.0\""),
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "failed to parse `Cargo.toml` manifest file\n\nTOML parse error at line 1, column 1\n  |\n1 | [package]\n  | ^^^^^^^^^\nmissing field `name`\n" }] })
    );
}

#[test]
fn invalid_manifest_missing_version() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token.publish_crate(
        PublishBuilder::new("foo", "1.0.0").custom_manifest("[package]\nname = \"foo\""),
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "failed to parse `Cargo.toml` manifest file\n\nTOML parse error at line 1, column 1\n  |\n1 | [package]\n  | ^^^^^^^^^\nmissing field `version`\n" }] })
    );
}

#[test]
fn invalid_rust_version() {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response =
        token.publish_crate(PublishBuilder::new("foo", "1.0.0").custom_manifest(
            "[package]\nname = \"foo\"\nversion = \"1.0.0\"\nrust-version = \"\"\n",
        ));
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "failed to parse `Cargo.toml` manifest file\n\ninvalid `rust-version` value" }] })
    );

    let response = token.publish_crate(PublishBuilder::new("foo", "1.0.0").custom_manifest(
        "[package]\nname = \"foo\"\nversion = \"1.0.0\"\nrust-version = \"1.0.0-beta.2\"\n",
    ));
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "failed to parse `Cargo.toml` manifest file\n\ninvalid `rust-version` value" }] })
    );
}
