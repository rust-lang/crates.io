use crate::TestApp;
use crate::util::{
    MockRequestExt, MockTokenUser, RequestHelper, Response,
    encode_session_header_with_explicit_epoch,
};

use crate::builders::PublishBuilder;
use crate::util::encode_session_header;
use crates_io::{models::User, schema::users};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
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
    let cookie = encode_session_header(session_key, -1, Some(0));

    let mut request = anon.request_builder(Method::GET, URL);
    request.header(header::COOKIE, &cookie);

    let error = anon.run::<()>(request).await;
    assert_eq!(error.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

// Ensure that cookies with older epochs than the user session epoch are rejected.
#[tokio::test(flavor = "multi_thread")]
async fn user_session_epoch() {
    // Note that, although we're creating a user, we're actually going to use
    // the anonymous user for all requests below so we control the session
    // cookie.
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;

    let User {
        id, session_epoch, ..
    } = user.as_model();
    let user_id = *id;
    let session_epoch = *session_epoch;

    // Set up a cookie with the default epoch.
    let session_key = app.as_inner().session_key();
    let default_cookie = encode_session_header(session_key, user_id, Some(session_epoch));

    // Also set up a cookie with no epoch, which should be functionally
    // equivalent to the above cookie.
    let epochless_cookie = encode_session_header(session_key, user_id, None);

    // Validate that a request succeeds for this user, both with and without an
    // epoch.
    let mut request = anon.get_request(URL);
    request.header(header::COOKIE, &default_cookie);

    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::OK);

    let mut request = anon.get_request(URL);
    request.header(header::COOKIE, &epochless_cookie);

    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::OK);

    // Validate that a request for a future epoch fails.
    let future_cookie = encode_session_header(session_key, user_id, Some(session_epoch + 1));
    let mut request = anon.get_request(URL);
    request.header(header::COOKIE, &future_cookie);

    let response = anon.run::<()>(request.clone()).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Validate that requests with various invalid epochs fail.
    for epoch in ["", "foobar", " ", " 0", "0 "] {
        let invalid_epoch_cookie =
            encode_session_header_with_explicit_epoch(session_key, user_id, Some(epoch));
        let mut request = anon.get_request(URL);
        request.header(header::COOKIE, &invalid_epoch_cookie);

        let response = anon.run::<()>(request.clone()).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    // Now increment the user epoch.
    diesel::update(users::table)
        .set(users::session_epoch.eq(session_epoch + 1))
        .filter(users::id.eq(id))
        .execute(&mut conn)
        .await
        .unwrap();

    // And validate that requests without an epoch, or with the default epoch
    // now fail.
    let mut request = anon.get_request(URL);
    request.header(header::COOKIE, &default_cookie);

    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let mut request = anon.get_request(URL);
    request.header(header::COOKIE, &epochless_cookie);

    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // But requests with the future epoch now succeed.
    let mut request = anon.get_request(URL);
    request.header(header::COOKIE, &future_cookie);

    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::OK);
}
