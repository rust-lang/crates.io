use crate::tests::util::{RequestHelper, TestApp};
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn site_metadata_includes_banner_message() {
    let (_app, anon) = TestApp::init()
        .with_config(|config| {
            config.db.primary.read_only_mode = true;
            config.banner_message = Some("Test banner message".to_string());
        })
        .empty()
        .await;

    let response = anon.get::<()>("/api/v1/site_metadata").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json());
}
