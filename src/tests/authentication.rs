use crate::{util::RequestHelper, TestApp};

use cargo_registry::middleware::current_user::TrustedUserId;

use conduit::{header, Handler, HandlerResult, Method, RequestExt, StatusCode};
use conduit_test::MockRequest;

static URL: &str = "/api/v1/me/updates";
static MUST_LOGIN: &[u8] =
    b"{\"errors\":[{\"detail\":\"must be logged in to perform that action\"}]}";
static INTERNAL_ERROR_NO_USER: &str =
    "user_id from cookie or token not found in database caused by NotFound";

fn call(app: &TestApp, mut request: MockRequest) -> HandlerResult {
    app.as_middleware().call(&mut request)
}

fn into_parts(response: HandlerResult) -> (StatusCode, std::borrow::Cow<'static, [u8]>) {
    use conduit_test::ResponseExt;

    let response = response.unwrap();
    (response.status(), response.into_cow())
}

#[test]
fn anonymous_user_unauthorized() {
    let (app, anon) = TestApp::init().empty();
    let request = anon.request_builder(Method::GET, URL);

    let (status, body) = into_parts(call(&app, request));
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, MUST_LOGIN);
}

#[test]
fn token_auth_cannot_find_token() {
    let (app, anon) = TestApp::init().empty();
    let mut request = anon.request_builder(Method::GET, URL);
    request.header(header::AUTHORIZATION, "cio1tkfake-token");

    let (status, body) = into_parts(call(&app, request));
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, MUST_LOGIN);
}

// Ensure that an unexpected authentication error is available for logging.  The user would see
// status 500 instead of 403 as in other authentication tests.  Due to foreign-key constraints in
// the database, it is not possible to implement this same test for a token.
#[test]
fn cookie_auth_cannot_find_user() {
    let (app, anon) = TestApp::init().empty();
    let mut request = anon.request_builder(Method::GET, URL);
    request.mut_extensions().insert(TrustedUserId(-1));

    let response = call(&app, request);
    let log_message = response.map(|_| ()).unwrap_err().to_string();
    assert_eq!(log_message, INTERNAL_ERROR_NO_USER);
}
