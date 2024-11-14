use crate::tests::builders::{CrateBuilder, PublishBuilder};
use crate::tests::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_with_readme() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_readme", "1.0.0").readme("hello world");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    assert_snapshot!(app.stored_files().await.join("\n"), @r"
    crates/foo_readme/foo_readme-1.0.0.crate
    index/fo/o_/foo_readme
    readmes/foo_readme/foo_readme-1.0.0.html
    rss/crates.xml
    rss/crates/foo_readme.xml
    rss/updates.xml
    ");
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_with_empty_readme() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_readme", "1.0.0").readme("");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    assert_snapshot!(app.stored_files().await.join("\n"), @r"
    crates/foo_readme/foo_readme-1.0.0.crate
    index/fo/o_/foo_readme
    rss/crates.xml
    rss/crates/foo_readme.xml
    rss/updates.xml
    ");
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_with_readme_and_plus_version() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_readme", "1.0.0+foo").readme("hello world");
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    assert_snapshot!(app.stored_files().await.join("\n"), @r"
    crates/foo_readme/foo_readme-1.0.0+foo.crate
    index/fo/o_/foo_readme
    readmes/foo_readme/foo_readme-1.0.0+foo.html
    rss/crates.xml
    rss/crates/foo_readme.xml
    rss/updates.xml
    ");
}

#[tokio::test(flavor = "multi_thread")]
async fn publish_after_removing_documentation() {
    let (app, anon, user, token) = TestApp::full().with_token();
    let mut conn = app.db_conn();
    let user = user.as_model();

    // 1. Start with a crate with no documentation
    CrateBuilder::new("docscrate", user.id)
        .version("0.2.0")
        .expect_build(&mut conn);

    // Verify that crates start without any documentation so the next assertion can *prove*
    // that it was the one that added the documentation
    let json = anon.show_crate("docscrate").await;
    assert_eq!(json.krate.documentation, None);

    // 2. Add documentation
    let crate_to_publish = PublishBuilder::new("docscrate", "0.2.1").documentation("http://foo.rs");
    let json = token.publish_crate(crate_to_publish).await.good();
    assert_eq!(json.krate.documentation, Some("http://foo.rs".to_owned()));

    // Ensure latest version also has the same documentation
    let json = anon.show_crate("docscrate").await;
    assert_eq!(json.krate.documentation, Some("http://foo.rs".to_owned()));

    // 3. Remove the documentation
    let crate_to_publish = PublishBuilder::new("docscrate", "0.2.2");
    let json = token.publish_crate(crate_to_publish).await.good();
    assert_eq!(json.krate.documentation, None);

    // Ensure latest version no longer has documentation
    let json = anon.show_crate("docscrate").await;
    assert_eq!(json.krate.documentation, None);
}
