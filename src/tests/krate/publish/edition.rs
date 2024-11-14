use crate::tests::builders::PublishBuilder;
use crate::tests::util::insta::{any_id_redaction, id_redaction};
use crate::tests::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_json_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn test_edition_is_saved() {
    let (_app, _, _, token) = TestApp::full().with_token().await;

    let manifest = r#"
        [package]
        name = "foo"
        version = "1.0.0"
        description = "description"
        license = "MIT"
        edition = "2021"
        rust-version = "1.0"
    "#;
    let pb = PublishBuilder::new("foo", "1.0.0").custom_manifest(manifest);
    let response = token.publish_crate(pb).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    let response = token.get::<()>("/api/v1/crates/foo/1.0.0").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".version.id" => any_id_redaction(),
        ".version.created_at" => "[datetime]",
        ".version.updated_at" => "[datetime]",
        ".version.published_by.id" => id_redaction(token.as_model().user_id),
        ".version.audit_actions[].time" => "[datetime]",
        ".version.audit_actions[].user.id" => id_redaction(token.as_model().user_id),
    });
}
