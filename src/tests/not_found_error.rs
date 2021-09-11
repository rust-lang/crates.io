use crate::{RequestHelper, TestApp};
use http::StatusCode;

#[test]
fn visiting_unknown_route_returns_404() {
    let (_, anon) = TestApp::init().empty();

    let response = anon.get::<()>("/does-not-exist");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "Not Found" }] })
    );
}

#[test]
fn visiting_unknown_api_route_returns_404() {
    let (_, anon) = TestApp::init().empty();

    let response = anon.get::<()>("/api/v1/does-not-exist");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "Not Found" }] })
    );
}
