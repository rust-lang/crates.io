use crate::tests::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn visiting_unknown_route_returns_404() {
    let (_, anon) = TestApp::init().empty();

    let response = anon.get::<()>("/does-not-exist").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Not Found"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn visiting_unknown_api_route_returns_404() {
    let (_, anon) = TestApp::init().empty();

    let response = anon.get::<()>("/api/v1/does-not-exist").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Not Found"}]}"#);
}
