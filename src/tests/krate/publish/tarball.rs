use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use bytes::{BufMut, BytesMut};
use claims::assert_ok;
use crates_io_tarball::TarballBuilder;
use googletest::prelude::*;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_wrong_files() {
    let (app, _, user) = TestApp::full().with_user().await;

    let builder = PublishBuilder::new("foo", "1.0.0")
        .add_file("foo-1.0.0/a", "")
        .add_file("bar-1.0.0/a", "");

    let response = user.publish_crate(builder).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid path found: bar-1.0.0/a"}]}"#);
    assert_that!(app.stored_files().await, is_empty());
}

async fn publish_tarball_with_entry(entry_type: tar::EntryType) -> String {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let tarball = {
        let mut builder = TarballBuilder::new();

        let mut header = tar::Header::new_gnu();
        assert_ok!(header.set_path("foo-1.1.0/bar"));
        header.set_size(0);
        header.set_entry_type(entry_type);
        if entry_type.is_hard_link() || entry_type.is_symlink() {
            assert_ok!(header.set_link_name("foo-1.1.0/another"));
        }
        if entry_type.is_gnu_sparse() {
            header.as_gnu_mut().unwrap().set_real_size(0);
        }
        header.set_cksum();
        assert_ok!(builder.as_mut().append(&header, &[][..]));

        builder.build()
    };

    let (json, _tarball) = PublishBuilder::new("foo", "1.1.0").build();
    let body = PublishBuilder::create_publish_body(&json, &tarball);

    let response = token.publish_crate(body).await;
    insta::allow_duplicates! {
        assert_snapshot!(response.status(), @"400 Bad Request");
    }
    assert_that!(app.stored_files().await, is_empty());

    response.text()
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_tarball_with_hard_link() {
    let response = publish_tarball_with_entry(tar::EntryType::hard_link()).await;
    assert_snapshot!(response, @r#"{"errors":[{"detail":"unexpected tar entry type Link found: foo-1.1.0/bar"}]}"#);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn new_krate_tarball_with_non_utf8_path() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let (app, _, _, token) = TestApp::full().with_token().await;

    let tarball = {
        let mut builder = TarballBuilder::new();

        let mut header = tar::Header::new_gnu();
        assert_ok!(header.set_path(OsStr::from_bytes(b"foo-1.1.0/\xff")));
        header.set_size(0);
        header.set_cksum();
        assert_ok!(builder.as_mut().append(&header, &[][..]));

        builder.build()
    };

    let (json, _tarball) = PublishBuilder::new("foo", "1.1.0").build();
    let body = PublishBuilder::create_publish_body(&json, &tarball);

    let response = token.publish_crate(body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid path found: foo-1.1.0/�"}]}"#);
    assert_that!(app.stored_files().await, is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_tarball_with_symlink() {
    let response = publish_tarball_with_entry(tar::EntryType::symlink()).await;
    assert_snapshot!(response, @r#"{"errors":[{"detail":"unexpected tar entry type Symlink found: foo-1.1.0/bar"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_tarball_with_chardev() {
    let response = publish_tarball_with_entry(tar::EntryType::character_special()).await;
    assert_snapshot!(response, @r#"{"errors":[{"detail":"unexpected tar entry type Char found: foo-1.1.0/bar"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_tarball_with_blockdev() {
    let response = publish_tarball_with_entry(tar::EntryType::block_special()).await;
    assert_snapshot!(response, @r#"{"errors":[{"detail":"unexpected tar entry type Block found: foo-1.1.0/bar"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_tarball_with_fifo() {
    let response = publish_tarball_with_entry(tar::EntryType::fifo()).await;
    assert_snapshot!(response, @r#"{"errors":[{"detail":"unexpected tar entry type Fifo found: foo-1.1.0/bar"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_tarball_with_contiguous_file() {
    let response = publish_tarball_with_entry(tar::EntryType::contiguous()).await;
    assert_snapshot!(response, @r#"{"errors":[{"detail":"unexpected tar entry type Continuous found: foo-1.1.0/bar"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_tarball_with_gnu_sparse_file() {
    let response = publish_tarball_with_entry(tar::EntryType::new(b'S')).await;
    assert_snapshot!(response, @r#"{"errors":[{"detail":"unexpected tar entry type GNUSparse found: foo-1.1.0/bar"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_tarball_with_unknown_entry_type() {
    let response = publish_tarball_with_entry(tar::EntryType::new(b'Z')).await;
    assert_snapshot!(response, @r#"{"errors":[{"detail":"unexpected tar entry type Other(90) found: foo-1.1.0/bar"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_body() {
    let (app, _, user) = TestApp::full().with_user().await;

    let response = user.publish_crate(&[] as &[u8]).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid metadata length"}]}"#);
    assert_that!(app.stored_files().await, is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn json_len_truncated() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let response = token.publish_crate(&[0u8, 0] as &[u8]).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid metadata length"}]}"#);
    assert_that!(app.stored_files().await, is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn json_bytes_truncated() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let response = token.publish_crate(&[100u8, 0, 0, 0, 0] as &[u8]).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid metadata length for remaining payload: 100"}]}"#);
    assert_that!(app.stored_files().await, is_empty());
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

    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid tarball length"}]}"#);
    assert_that!(app.stored_files().await, is_empty());
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
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid tarball length for remaining payload: 100"}]}"#);
    assert_that!(app.stored_files().await, is_empty());
}
