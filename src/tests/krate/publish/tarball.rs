use crate::tests::builders::PublishBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use bytes::{BufMut, BytesMut};
use crates_io_tarball::TarballBuilder;
use googletest::prelude::*;
use http::StatusCode;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_wrong_files() {
    let (app, _, user) = TestApp::full().with_user().await;

    let builder = PublishBuilder::new("foo", "1.0.0")
        .add_file("foo-1.0.0/a", "")
        .add_file("bar-1.0.0/a", "");

    let response = user.publish_crate(builder).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid path found: bar-1.0.0/a"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_tarball_with_hard_links() {
    let (app, _, _, token) = TestApp::full().with_token().await;

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

    let response = token.publish_crate(body).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"unexpected symlink or hard link found: foo-1.1.0/bar"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_body() {
    let (app, _, user) = TestApp::full().with_user().await;

    let response = user.publish_crate(&[] as &[u8]).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid metadata length"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn json_len_truncated() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let response = token.publish_crate(&[0u8, 0] as &[u8]).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid metadata length"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn json_bytes_truncated() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let response = token.publish_crate(&[100u8, 0, 0, 0, 0] as &[u8]).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid metadata length for remaining payload: 100"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn tarball_len_truncated() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let json = br#"{ "name": "foo", "vers": "1.0.0" }"#;

    let mut bytes = BytesMut::new();
    bytes.put_u32_le(json.len() as u32);
    bytes.put_slice(json);
    bytes.put_u8(0);
    bytes.put_u8(0);

    let response = token.publish_crate(bytes.freeze()).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid tarball length"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn tarball_bytes_truncated() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let json = br#"{ "name": "foo", "vers": "1.0.0" }"#;

    let mut bytes = BytesMut::new();
    bytes.put_u32_le(json.len() as u32);
    bytes.put_slice(json);
    bytes.put_u32_le(100);
    bytes.put_u8(0);

    let response = token.publish_crate(bytes.freeze()).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid tarball length for remaining payload: 100"}]}"#);
    assert_that!(app.stored_files().await, empty());
}
