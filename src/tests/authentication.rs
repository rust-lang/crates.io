use crate::TestApp;
use crate::util::{MockRequestExt, MockTokenUser, RequestHelper, Response};

use crate::builders::PublishBuilder;
use crate::util::encode_session_header;
use http::{Method, StatusCode, header};
use insta::assert_snapshot;

static URL: &str = "/api/v1/me/updates";

#[tokio::test(flavor = "multi_thread")]
async fn anonymous_user_unauthorized() {
    let (_, anon) = TestApp::init().empty().await;
    let response: Response<()> = anon.get(URL).await;

    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn token_auth_cannot_find_token() {
    let (app, _anon) = TestApp::full().empty().await;

    let client = MockTokenUser::with_auth_header("cio1tkfake-token".to_string(), app.clone());
    let pb = PublishBuilder::new("foo", "1.0.0");
    let response = client.publish_crate(pb).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"authentication failed"}]}"#);
}

// Ensure that an unexpected authentication error is available for logging.  The user would see
// status 500 instead of 403 as in other authentication tests.  Due to foreign-key constraints in
// the database, it is not possible to implement this same test for a token.
#[tokio::test(flavor = "multi_thread")]
async fn cookie_auth_cannot_find_user() {
    let (app, anon) = TestApp::init().empty().await;

    let session_key = app.as_inner().session_key();
    let cookie = encode_session_header(session_key, -1);

    let mut request = anon.request_builder(Method::GET, URL);
    request.header(header::COOKIE, &cookie);

    let error = anon.run::<()>(request).await;
    assert_eq!(error.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
