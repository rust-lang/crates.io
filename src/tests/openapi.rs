use crate::util::{RequestHelper, TestApp};
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn test_openapi_snapshot() {
    let (_app, anon) = TestApp::init().empty().await;

    let response = anon.get::<()>("/api/openapi.json").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json());
}
