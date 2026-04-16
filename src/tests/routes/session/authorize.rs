use crates_io::controllers::session::SESSION_KEY_OAUTH_STATE;
use crate::util::{MockRequestExt, RequestHelper, TestApp, encode_session_header_with_data};
use http::header;
use insta::assert_snapshot;
use std::collections::HashMap;

#[tokio::test(flavor = "multi_thread")]
async fn access_token_needs_data() {
    let (_, anon) = TestApp::init().empty().await;
    let response = anon.get::<()>("/api/private/session/authorize").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize query string: missing field `code`"}]}"#);
}

/// Calling authorize with `?code=x&state=y` but no session cookie (no
/// `oauth_state` stored) must be rejected with 400 Bad Request.
#[tokio::test(flavor = "multi_thread")]
async fn authorize_with_no_session_state_returns_400() {
    let (_, anon) = TestApp::init().empty().await;
    let response = anon
        .get::<()>("/api/private/session/authorize?code=xcode&state=xstate")
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid state parameter"}]}"#);
}

/// A session cookie that contains `oauth_state` with malformed JSON (not a
/// valid `OAuthStatePayload`) must be rejected with 400 Bad Request.
#[tokio::test(flavor = "multi_thread")]
async fn authorize_with_malformed_session_state_returns_400() {
    let (app, anon) = TestApp::init().empty().await;
    let session_key = app.as_inner().session_key();

    let mut data = HashMap::new();
    data.insert(SESSION_KEY_OAUTH_STATE.into(), "this is not valid json".into());
    let cookie = encode_session_header_with_data(&session_key, data);

    let mut request = anon.get_request("/api/private/session/authorize?code=xcode&state=xstate");
    request.header(header::COOKIE, &cookie);
    let response = anon.run::<()>(request).await;

    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid state parameter"}]}"#);
}

/// If the `state` query parameter does not match the CSRF token stored in the
/// session, the request must be rejected with 400 Bad Request.
#[tokio::test(flavor = "multi_thread")]
async fn authorize_with_wrong_csrf_state_returns_400() {
    let (app, anon) = TestApp::init().empty().await;
    let session_key = app.as_inner().session_key();

    // Store a valid JSON payload with state = "correct_csrf_token".
    let payload = r#"{"state":"correct_csrf_token","provider":"github"}"#;
    let mut data = HashMap::new();
    data.insert(SESSION_KEY_OAUTH_STATE.into(), payload.into());
    let cookie = encode_session_header_with_data(&session_key, data);

    // Call authorize with a DIFFERENT state value.
    let mut request =
        anon.get_request("/api/private/session/authorize?code=xcode&state=wrong_csrf_token");
    request.header(header::COOKIE, &cookie);
    let response = anon.run::<()>(request).await;

    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid state parameter"}]}"#);
}

/// If the `oauth_state` session payload names an unknown provider, the
/// authorize endpoint must reject the request with 400 Bad Request.
#[tokio::test(flavor = "multi_thread")]
async fn authorize_with_unknown_provider_in_session_returns_400() {
    // No providers registered in the empty registry.
    let (app, anon) = TestApp::init().empty().await;
    let session_key = app.as_inner().session_key();

    // The CSRF token in the payload matches the `state` query param, but the
    // provider name is not registered.
    let payload = r#"{"state":"mycsrf","provider":"nonexistent_provider"}"#;
    let mut data = HashMap::new();
    data.insert(SESSION_KEY_OAUTH_STATE.into(), payload.into());
    let cookie = encode_session_header_with_data(&session_key, data);

    let mut request =
        anon.get_request("/api/private/session/authorize?code=xcode&state=mycsrf");
    request.header(header::COOKIE, &cookie);
    let response = anon.run::<()>(request).await;

    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"unknown oauth provider in session"}]}"#);
}
