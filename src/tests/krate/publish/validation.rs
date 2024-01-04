use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use crates_io::models::krate::MAX_NAME_LENGTH;
use googletest::prelude::*;
use http::StatusCode;
use insta::assert_json_snapshot;

#[test]
fn empty_json() {
    let (app, _, _, token) = TestApp::full().with_token();

    let (_json, tarball) = PublishBuilder::new("foo", "1.0.0").build();
    let body = PublishBuilder::create_publish_body("{}", &tarball);

    let response = token.publish_crate(body);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn invalid_names() {
    let (app, _, _, token) = TestApp::full().with_token();

    let bad_name = |name: &str| {
        let crate_to_publish = PublishBuilder::new(name, "1.0.0");
        let response = token.publish_crate(crate_to_publish);
        assert_eq!(response.status(), StatusCode::OK);
        assert_json_snapshot!(response.json());
    };

    bad_name("");
    bad_name("foo bar");
    bad_name(&"a".repeat(MAX_NAME_LENGTH + 1));
    bad_name("snow☃");
    bad_name("áccênts");

    bad_name("std");
    bad_name("STD");
    bad_name("compiler-rt");
    bad_name("compiler_rt");
    bad_name("coMpiLer_Rt");

    assert_that!(app.stored_files(), empty());
}

#[test]
fn invalid_version() {
    let (app, _, _, token) = TestApp::full().with_token();

    let (json, tarball) = PublishBuilder::new("foo", "1.0.0").build();
    let new_json = json.replace(r#""vers":"1.0.0""#, r#""vers":"broken""#);
    assert_ne!(json, new_json);
    let body = PublishBuilder::create_publish_body(&new_json, &tarball);

    let response = token.publish_crate(body);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn license_and_description_required() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_metadata", "1.1.0")
        .unset_license()
        .unset_description();

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());

    let crate_to_publish = PublishBuilder::new("foo_metadata", "1.1.0").unset_description();

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());

    let crate_to_publish = PublishBuilder::new("foo_metadata", "1.1.0")
        .unset_license()
        .license_file("foo")
        .unset_description();

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());

    assert_that!(app.stored_files(), empty());
}

#[test]
fn invalid_license() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response =
        token.publish_crate(PublishBuilder::new("foo", "1.0.0").license("MIT AND foobar"));
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn invalid_urls() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token.publish_crate(
        PublishBuilder::new("foo", "1.0.0").documentation("javascript:alert('boom')"),
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());

    assert_that!(app.stored_files(), empty());
}
