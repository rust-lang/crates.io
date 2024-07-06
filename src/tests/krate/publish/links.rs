use crate::builders::PublishBuilder;
use crate::util::insta::{self, assert_json_snapshot};
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;

#[tokio::test(flavor = "multi_thread")]
async fn test_crate_with_links_field() {
    let (app, anon, _, token) = TestApp::full().with_token();

    let manifest = r#"
    [package]
    name = "foo"
    version = "1.0.0"
    description = "foo?!"
    license = "MIT"
    links = "git2"
    "#;

    let pb = PublishBuilder::new("foo", "1.0.0").custom_manifest(manifest);
    let response = token.publish_crate(pb).await;
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });
    assert_eq!(response.status(), StatusCode::OK);

    let response = anon.get::<()>("/api/v1/crates/foo/1.0.0").await;
    assert_json_snapshot!(response.json(), {
        ".version.id" => insta::any_id_redaction(),
        ".version.created_at" => "[datetime]",
        ".version.updated_at" => "[datetime]",
        ".version.audit_actions[].time" => "[datetime]",
        ".version.published_by.id" => insta::any_id_redaction(),
    });
    assert_eq!(response.status(), StatusCode::OK);

    let crates = app.crates_from_index_head("foo");
    assert_json_snapshot!(crates);
}
