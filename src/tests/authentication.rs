use crate::util::{RequestHelper, Response};
use crate::TestApp;

use crate::util::encode_session_header;
use cargo_registry::models::persistent_session::SessionCookie;
use cargo_registry::util::token::SecureToken;
use cargo_registry::util::token::SecureTokenKind;
use conduit::{header, Body, Method, StatusCode};

static URL: &str = "/api/v1/me/updates";
static MUST_LOGIN: &[u8] = br#"{"errors":[{"detail":"must be logged in to perform that action"}]}"#;
static INTERNAL_ERROR_NO_USER: &str =
    "user_id from cookie not found in database caused by NotFound";

#[test]
fn persistent_session_user() {
    let (app, _) = TestApp::init().empty();
    let user = app.db_new_user("user1").with_session();
    let request = user.request_builder(Method::GET, URL);
    let response: Response<Body> = user.run(request);
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn persistent_session_revoked_after_logout() {
    let (app, _) = TestApp::init().empty();
    let user = app.db_new_user("user1").with_session();
    let request = user.request_builder(Method::GET, URL);
    let response: Response<Body> = user.run(request);
    assert_eq!(response.status(), StatusCode::OK);

    // Logout
    let request = user.request_builder(Method::DELETE, "/api/private/session");
    let response: Response<Body> = user.run(request);
    assert_eq!(response.status(), StatusCode::OK);

    // Now this request should fail since we logged out.
    let request = user.request_builder(Method::GET, URL);
    let response: Response<Body> = user.run(request);
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn incorrect_session_is_forbidden() {
    let (_, anon) = TestApp::init().empty();

    let token = SecureToken::generate(SecureTokenKind::Session);
    // Create a cookie that isn't in the database.
    let cookie = SessionCookie::new(123, token.plaintext().to_string())
        .build(false)
        .to_string();
    let mut request = anon.request_builder(Method::GET, URL);
    request.header(header::COOKIE, &cookie);
    let response: Response<Body> = anon.run(request);
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response.into_json(),
        json!({"errors": [{"detail": "must be logged in to perform that action"}]})
    );
}

#[test]
fn cookie_user() {
    let (_, _, cookie_user) = TestApp::init().with_user();
    let request = cookie_user.request_builder(Method::GET, URL);

    let response: Response<Body> = cookie_user.run(request);
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn anonymous_user_unauthorized() {
    let (_, anon) = TestApp::init().empty();
    let response: Response<Body> = anon.get(URL);

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response.into_json().to_string().as_bytes(), MUST_LOGIN);
}

#[test]
fn token_auth_cannot_find_token() {
    let (_, anon) = TestApp::init().empty();
    let mut request = anon.request_builder(Method::GET, URL);
    request.header(header::AUTHORIZATION, "cio1tkfake-token");
    let response: Response<Body> = anon.run(request);

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response.into_json().to_string().as_bytes(), MUST_LOGIN);
}

// Ensure that an unexpected authentication error is available for logging.  The user would see
// status 500 instead of 403 as in other authentication tests.  Due to foreign-key constraints in
// the database, it is not possible to implement this same test for a token.
#[test]
fn cookie_auth_cannot_find_user() {
    let (app, anon) = TestApp::init().empty();

    let session_key = &app.as_inner().session_key();
    let cookie = encode_session_header(session_key, -1);

    let mut request = anon.request_builder(Method::GET, URL);
    request.header(header::COOKIE, &cookie);

    let error = anon.run_err(request);
    assert_eq!(error.to_string(), INTERNAL_ERROR_NO_USER);
}
