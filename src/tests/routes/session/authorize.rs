use crate::util::{RequestHelper, TestApp};
use http::StatusCode;

#[tokio::test(flavor = "multi_thread")]
async fn access_token_needs_data() {
    let (_, anon) = TestApp::init().empty();
    let response = anon.async_get::<()>("/api/private/session/authorize").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": "Failed to deserialize query string: missing field `code`" }] })
    );
}
