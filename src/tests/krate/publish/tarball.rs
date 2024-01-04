use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use crates_io_tarball::TarballBuilder;
use googletest::prelude::*;
use http::StatusCode;
use insta::assert_json_snapshot;

#[test]
fn new_krate_wrong_files() {
    let (app, _, user) = TestApp::full().with_user();

    let builder = PublishBuilder::new("foo", "1.0.0")
        .add_file("foo-1.0.0/a", "")
        .add_file("bar-1.0.0/a", "");

    let response = user.publish_crate(builder);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": "invalid path found: bar-1.0.0/a" }] })
    );

    assert_that!(app.stored_files(), empty());
}

#[test]
fn new_krate_tarball_with_hard_links() {
    let (app, _, _, token) = TestApp::full().with_token();

    let tarball = {
        let mut builder = TarballBuilder::new();

        let mut header = tar::Header::new_gnu();
        assert_ok!(header.set_path("foo-1.1.0/bar"));
        header.set_size(0);
        header.set_entry_type(tar::EntryType::hard_link());
        assert_ok!(header.set_link_name("foo-1.1.0/another"));
        header.set_cksum();
        assert_ok!(builder.as_mut().append(&header, &[][..]));

        builder.build()
    };

    let (json, _tarball) = PublishBuilder::new("foo", "1.1.0").build();
    let body = PublishBuilder::create_publish_body(&json, &tarball);

    let response = token.publish_crate(body);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn empty_body() {
    let (app, _, user) = TestApp::full().with_user();

    let response = user.publish_crate(&[] as &[u8]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn json_len_truncated() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token.publish_crate(&[0u8, 0] as &[u8]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn json_bytes_truncated() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token.publish_crate(&[100u8, 0, 0, 0, 0] as &[u8]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn tarball_len_truncated() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token.publish_crate(&[2, 0, 0, 0, b'{', b'}', 0, 0] as &[u8]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn tarball_bytes_truncated() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token.publish_crate(&[2, 0, 0, 0, b'{', b'}', 100, 0, 0, 0, 0] as &[u8]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}
