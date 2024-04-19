use crate::{RequestHelper, TestApp};
use http::StatusCode;

#[tokio::test(flavor = "multi_thread")]
async fn visiting_unknown_route_returns_404() {
    let (_, anon) = TestApp::init().empty();

    let response = anon.async_get::<()>("/does-not-exist").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": "Not Found" }] })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn visiting_unknown_api_route_returns_404() {
    let (_, anon) = TestApp::init().empty();

    let response = anon.async_get::<()>("/api/v1/does-not-exist").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": "Not Found" }] })
    );
}
