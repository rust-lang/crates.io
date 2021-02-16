use crate::{util::RequestHelper, TestApp};

use crate::util::encode_session_header;
use cargo_registry::controllers::util::auth_cookie;
use conduit::{header, Handler, HandlerResult, Method, StatusCode};
use conduit_test::MockRequest;
use serde_json::Value;

static URL: &str = "/api/v1/me/updates";
static INTERNAL_ERROR_NO_USER: &str =
    "user_id from cookie not found in database caused by NotFound";

fn call(app: &TestApp, mut request: MockRequest) -> HandlerResult {
    app.as_middleware().call(&mut request)
}

#[test]
fn session_user() {
    let token = "some-random-token";

    let (app, _) = TestApp::init().empty();
    let session_user = app.db_new_user("user1").with_session(token);
    let request = session_user.request_builder(Method::GET, URL);

    let response = session_user.run::<Value>(request);
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn cookie_user() {
    let (_app, _, cookie_user) = TestApp::init().with_user();
    let request = cookie_user.request_builder(Method::GET, URL);

    let response = cookie_user.run::<Value>(request);
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn token_user() {
    let (_app, _, _, token_user) = TestApp::init().with_token();
    let request = token_user.request_builder(Method::GET, URL);

    let response = token_user.run::<Value>(request);
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn anonymous_user_unauthorized() {
    let (_app, anon) = TestApp::init().empty();
    let request = anon.request_builder(Method::GET, URL);

    let response = anon.run::<Value>(request);
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": "must be logged in to perform that action" }] })
    );
}

#[test]
fn session_auth_cannot_find_token() {
    let cookie = auth_cookie("some-unknown-token", false).to_string();

    let (_app, anon) = TestApp::init().empty();
    let mut request = anon.request_builder(Method::GET, URL);
    request.header(header::COOKIE, &cookie);

    let response = anon.run::<Value>(request);
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": "must be logged in to perform that action" }] })
    );
}

#[test]
fn token_auth_cannot_find_token() {
    let (_app, anon) = TestApp::init().empty();
    let mut request = anon.request_builder(Method::GET, URL);
    request.header(header::AUTHORIZATION, "cio1tkfake-token");

    let response = anon.run::<Value>(request);
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": "must be logged in to perform that action" }] })
    );
}

// Ensure that an unexpected authentication error is available for logging.  The user would see
// status 500 instead of 403 as in other authentication tests.  Due to foreign-key constraints in
// the database, it is not possible to implement this same test for a token.
#[test]
fn cookie_auth_cannot_find_user() {
    let (app, anon) = TestApp::init().empty();

    let session_key = &app.as_inner().session_key;
    let cookie = encode_session_header(session_key, -1);

    let mut request = anon.request_builder(Method::GET, URL);
    request.header(header::COOKIE, &cookie);

    let response = call(&app, request);
    let log_message = response.map(|_| ()).unwrap_err().to_string();
    assert_eq!(log_message, INTERNAL_ERROR_NO_USER);
}
