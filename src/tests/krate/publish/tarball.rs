use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use crates_io_tarball::TarballBuilder;
use googletest::prelude::*;
use http::StatusCode;
use insta::assert_json_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_wrong_files() {
    let (app, _, user) = TestApp::full().with_user();

    let builder = PublishBuilder::new("foo", "1.0.0")
        .add_file("foo-1.0.0/a", "")
        .add_file("bar-1.0.0/a", "");

    let response = user.async_publish_crate(builder).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": "invalid path found: bar-1.0.0/a" }] })
    );

    assert_that!(app.async_stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_tarball_with_hard_links() {
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

    let response = token.async_publish_crate(body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
    assert_that!(app.async_stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_body() {
    let (app, _, user) = TestApp::full().with_user();

    let response = user.async_publish_crate(&[] as &[u8]).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
    assert_that!(app.async_stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn json_len_truncated() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token.async_publish_crate(&[0u8, 0] as &[u8]).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
    assert_that!(app.async_stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn json_bytes_truncated() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token
        .async_publish_crate(&[100u8, 0, 0, 0, 0] as &[u8])
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
    assert_that!(app.async_stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn tarball_len_truncated() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token
        .async_publish_crate(&[2, 0, 0, 0, b'{', b'}', 0, 0] as &[u8])
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
    assert_that!(app.async_stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn tarball_bytes_truncated() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token
        .async_publish_crate(&[2, 0, 0, 0, b'{', b'}', 100, 0, 0, 0, 0] as &[u8])
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());
    assert_that!(app.async_stored_files().await, empty());
}
