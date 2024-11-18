use crate::tests::builders::{CrateBuilder, PublishBuilder, VersionBuilder};
use crate::tests::util::{RequestHelper, TestApp};
use diesel::prelude::*;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn show() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn();
    let user = user.as_model();

    use crate::schema::versions;
    use diesel::{update, ExpressionMethods};

    CrateBuilder::new("foo_show", user.id)
        .description("description")
        .documentation("https://example.com")
        .homepage("http://example.com")
        .version(VersionBuilder::new("1.0.0"))
        .version(VersionBuilder::new("0.5.0"))
        .version(VersionBuilder::new("0.5.1"))
        .keyword("kw1")
        .downloads(20)
        .recent_downloads(10)
        .expect_build(&mut conn);

    // Make version 1.0.0 mimic a version published before we started recording who published
    // versions
    let none: Option<i32> = None;
    update(versions::table)
        .filter(versions::num.eq("1.0.0"))
        .set(versions::published_by.eq(none))
        .execute(&mut conn)
        .unwrap();

    let response = anon.get::<()>("/api/v1/crates/foo_show").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
        ".keywords[].created_at" => "[datetime]",
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn show_minimal() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn();
    let user = user.as_model();

    CrateBuilder::new("foo_show_minimal", user.id)
        .description("description")
        .documentation("https://example.com")
        .homepage("http://example.com")
        .version(VersionBuilder::new("1.0.0"))
        .version(VersionBuilder::new("0.5.0"))
        .version(VersionBuilder::new("0.5.1"))
        .keyword("kw1")
        .downloads(20)
        .recent_downloads(10)
        .expect_build(&mut conn);

    let response = anon
        .get::<()>("/api/v1/crates/foo_show_minimal?include=")
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn show_all_yanked() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn();
    let user = user.as_model();

    CrateBuilder::new("foo_show", user.id)
        .description("description")
        .documentation("https://example.com")
        .homepage("http://example.com")
        .version(VersionBuilder::new("1.0.0").yanked(true))
        .version(VersionBuilder::new("0.5.0").yanked(true))
        .keyword("kw1")
        .downloads(20)
        .recent_downloads(10)
        .expect_build(&mut conn);

    let response = anon.get::<()>("/api/v1/crates/foo_show").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
        ".keywords[].created_at" => "[datetime]",
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn test_missing() {
    let (_, anon) = TestApp::init().empty();

    let response = anon.get::<()>("/api/v1/crates/missing").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `missing` does not exist"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn version_size() {
    let (_, _, user) = TestApp::full().with_user().await;

    let crate_to_publish = PublishBuilder::new("foo_version_size", "1.0.0");
    user.publish_crate(crate_to_publish).await.good();

    // Add a file to version 2 so that it's a different size than version 1
    let crate_to_publish = PublishBuilder::new("foo_version_size", "2.0.0")
        .add_file("foo_version_size-2.0.0/big", "a");
    user.publish_crate(crate_to_publish).await.good();

    let crate_json = user.show_crate("foo_version_size").await;

    let version1 = crate_json
        .versions
        .as_ref()
        .unwrap()
        .iter()
        .find(|v| v.num == "1.0.0")
        .expect("Could not find v1.0.0");
    assert_eq!(version1.crate_size, 158);

    let version2 = crate_json
        .versions
        .as_ref()
        .unwrap()
        .iter()
        .find(|v| v.num == "2.0.0")
        .expect("Could not find v2.0.0");
    assert_eq!(version2.crate_size, 184);
}

#[tokio::test(flavor = "multi_thread")]
async fn block_bad_documentation_url() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn();
    let user = user.as_model();

    CrateBuilder::new("foo_bad_doc_url", user.id)
        .documentation("http://rust-ci.org/foo/foo_bad_doc_url/doc/foo_bad_doc_url/")
        .expect_build(&mut conn);

    let json = anon.show_crate("foo_bad_doc_url").await;
    assert_eq!(json.krate.documentation, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_new_name() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn();

    CrateBuilder::new("new", user.as_model().id).expect_build(&mut conn);

    let response = anon.get::<()>("/api/v1/crates/new?include=").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });
}
