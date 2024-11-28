use crate::tests::builders::{CrateBuilder, PublishBuilder};
use crate::tests::util::{RequestHelper, TestApp};
use crates_io_tarball::TarballBuilder;
use flate2::Compression;
use googletest::prelude::*;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn tarball_between_default_axum_limit_and_max_upload_size() {
    let max_upload_size = 5 * 1024 * 1024;
    let (app, _, _, token) = TestApp::full()
        .with_config(|config| {
            config.max_upload_size = max_upload_size;
            config.max_unpack_size = max_upload_size as u64;
        })
        .with_token()
        .await;

    let tarball = {
        let mut builder = TarballBuilder::new();

        let data = b"[package]\nname = \"foo\"\nversion = \"1.1.0\"\ndescription = \"description\"\nlicense = \"MIT\"\n" as &[_];

        let mut header = tar::Header::new_gnu();
        assert_ok!(header.set_path("foo-1.1.0/Cargo.toml"));
        header.set_size(data.len() as u64);
        header.set_cksum();
        assert_ok!(builder.as_mut().append(&header, data));

        // `data` is smaller than `max_upload_size`, but bigger than the regular request body limit
        let data = vec![b'a'; 3 * 1024 * 1024];

        let mut header = tar::Header::new_gnu();
        assert_ok!(header.set_path("foo-1.1.0/big-file.txt"));
        header.set_size(data.len() as u64);
        header.set_cksum();
        assert_ok!(builder.as_mut().append(&header, data.as_slice()));

        // We explicitly disable compression to be able to influence the final tarball size
        builder.build_with_compression(Compression::none())
    };

    let (json, _tarball) = PublishBuilder::new("foo", "1.1.0").build();
    let body = PublishBuilder::create_publish_body(&json, &tarball);

    let response = token.publish_crate(body).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });
    assert_snapshot!(app.stored_files().await.join("\n"), @r"
    crates/foo/foo-1.1.0.crate
    index/3/f/foo
    rss/crates.xml
    rss/crates/foo.xml
    rss/updates.xml
    ");
}

#[tokio::test(flavor = "multi_thread")]
async fn tarball_bigger_than_max_upload_size() {
    let max_upload_size = 5 * 1024 * 1024;
    let (app, _, _, token) = TestApp::full()
        .with_config(|config| {
            config.max_upload_size = max_upload_size;
            config.max_unpack_size = max_upload_size as u64;
        })
        .with_token()
        .await;

    let tarball = {
        // `data` is bigger than `max_upload_size`
        let data = vec![b'a'; 6 * 1024 * 1024];

        let mut builder = TarballBuilder::new();

        let mut header = tar::Header::new_gnu();
        assert_ok!(header.set_path("foo-1.1.0/Cargo.toml"));
        header.set_size(data.len() as u64);
        header.set_cksum();
        assert_ok!(builder.as_mut().append(&header, data.as_slice()));

        // We explicitly disable compression to be able to influence the final tarball size
        builder.build_with_compression(Compression::none())
    };

    let (json, _tarball) = PublishBuilder::new("foo", "1.1.0").build();
    let body = PublishBuilder::create_publish_body(&json, &tarball);

    let response = token.publish_crate(body).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"max upload size is: 5242880"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_gzip_bomb() {
    let (app, _, _, token) = TestApp::full()
        .with_config(|config| {
            config.max_upload_size = 3000;
            config.max_unpack_size = 2000;
        })
        .with_token()
        .await;

    let body = vec![0; 512 * 1024];
    let crate_to_publish = PublishBuilder::new("foo", "1.1.0").add_file("foo-1.1.0/a", body);

    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"uploaded tarball is malformed or too large when decompressed"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_too_big() {
    let (app, _, user) = TestApp::full()
        .with_config(|config| {
            config.max_upload_size = 3000;
            config.max_unpack_size = 2000;
        })
        .with_user()
        .await;

    let builder =
        PublishBuilder::new("foo_big", "1.0.0").add_file("foo_big-1.0.0/big", vec![b'a'; 2000]);

    let response = user.publish_crate(builder).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"uploaded tarball is malformed or too large when decompressed"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_too_big_but_whitelisted() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo_whitelist", user.as_model().id)
        .max_upload_size(2_000_000)
        .expect_build(&mut conn)
        .await;

    let crate_to_publish = PublishBuilder::new("foo_whitelist", "1.1.0")
        .add_file("foo_whitelist-1.1.0/big", vec![b'a'; 2000]);

    token.publish_crate(crate_to_publish).await.good();

    assert_snapshot!(app.stored_files().await.join("\n"), @r"
    crates/foo_whitelist/foo_whitelist-1.1.0.crate
    index/fo/o_/foo_whitelist
    rss/crates/foo_whitelist.xml
    rss/updates.xml
    ");
}
