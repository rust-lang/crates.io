use crate::builders::{CrateBuilder, PublishBuilder};
use crate::util::{RequestHelper, TestApp};
use crates_io::schema::versions_published_by;
use diesel::{QueryDsl, RunQueryDsl};
use googletest::prelude::*;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn new_krate() {
    let (app, _, user) = TestApp::full().with_user();

    let crate_to_publish = PublishBuilder::new("foo_new", "1.0.0");
    let response = user.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    let crates = app.crates_from_index_head("foo_new");
    assert_json_snapshot!(crates);

    assert_snapshot!(app.stored_files().await.join("\n"), @r###"
    crates/foo_new/foo_new-1.0.0.crate
    index/fo/o_/foo_new
    rss/crates.xml
    rss/crates/foo_new.xml
    rss/updates.xml
    "###);

    app.db(|conn| {
        let email: String = versions_published_by::table
            .select(versions_published_by::email)
            .first(conn)
            .unwrap();
        assert_eq!(email, "foo@example.com");
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_with_token() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_new", "1.0.0");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    assert_snapshot!(app.stored_files().await.join("\n"), @r###"
    crates/foo_new/foo_new-1.0.0.crate
    index/fo/o_/foo_new
    rss/crates.xml
    rss/crates/foo_new.xml
    rss/updates.xml
    "###);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_weird_version() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_weird", "0.0.0-pre");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    assert_snapshot!(app.stored_files().await.join("\n"), @r###"
    crates/foo_weird/foo_weird-0.0.0-pre.crate
    index/fo/o_/foo_weird
    rss/crates.xml
    rss/crates/foo_weird.xml
    rss/updates.xml
    "###);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_twice() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_twice", "0.99.0");
    token.publish_crate(crate_to_publish).await.good();

    let crate_to_publish =
        PublishBuilder::new("foo_twice", "2.0.0").description("2.0.0 description");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    let crates = app.crates_from_index_head("foo_twice");
    assert_json_snapshot!(crates);

    assert_snapshot!(app.stored_files().await.join("\n"), @r###"
    crates/foo_twice/foo_twice-0.99.0.crate
    crates/foo_twice/foo_twice-2.0.0.crate
    index/fo/o_/foo_twice
    rss/crates.xml
    rss/crates/foo_twice.xml
    rss/updates.xml
    "###);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_duplicate_version() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database and then we'll try to publish the same version
        CrateBuilder::new("foo_dupe", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo_dupe", "1.0.0");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_json_snapshot!(response.json());

    assert_that!(app.stored_files().await, empty());
}
