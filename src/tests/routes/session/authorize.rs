use crate::util::{RequestHelper, TestApp};
use http::StatusCode;

#[test]
fn access_token_needs_data() {
    let (_, anon) = TestApp::init().empty();
    let response = anon.get::<()>("/api/private/session/authorize");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": "Failed to deserialize query string: missing field `code`" }] })
    );
}
