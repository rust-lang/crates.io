use crate::util::{MockRequestExt, RequestHelper, TestApp};
use chrono::{Duration, Utc};
use cookie::{Cookie, CookieJar, Key};
use crates_io::controllers::session::{PENDING_SIGNUP_KEY, PendingSignup};
use crates_io_github::GitHubUser;
use http::{Method, header};
use insta::assert_snapshot;
use serde_json::Value;
use std::collections::HashMap;

/// Encodes arbitrary session values as a signed cookie for an integration test request.
fn encode_session_header(session_key: &Key, values: HashMap<String, String>) -> String {
    let encoded = crates_io_session::encode(&values);
    let cookie = Cookie::build((crates_io_session::COOKIE_NAME, encoded));
    let mut jar = CookieJar::new();
    jar.signed_mut(session_key).add(cookie);
    jar.get(crates_io_session::COOKIE_NAME).unwrap().to_string()
}

/// Builds pending-signup state with a configurable creation time.
fn pending_signup(created_at: chrono::DateTime<Utc>) -> PendingSignup {
    PendingSignup {
        github_user: GitHubUser {
            avatar_url: Some("https://avatars.example.com/42".into()),
            email: Some("ghost@example.com".into()),
            id: 42,
            login: "ghost".into(),
            name: Some("Ghost".into()),
        },
        encrypted_token: vec![1, 2, 3],
        created_at,
    }
}

/// Encodes pending-signup state as a signed session cookie.
fn pending_signup_header(app: &TestApp, created_at: chrono::DateTime<Utc>) -> String {
    let pending_signup = serde_json::to_string(&pending_signup(created_at)).unwrap();
    let values = HashMap::from([(PENDING_SIGNUP_KEY.into(), pending_signup)]);
    encode_session_header(app.as_inner().session_key(), values)
}

#[tokio::test(flavor = "multi_thread")]
async fn get_returns_safe_signup_details() {
    let (app, anon) = TestApp::init().empty().await;
    let cookie = pending_signup_header(&app, Utc::now());
    let mut request = anon.request_builder(Method::GET, "/api/private/session/signup");
    request.header(header::COOKIE, &cookie);

    let response = anon.run::<Value>(request).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.json(), @r#"{"signup":{"email":"ghost@example.com","login":"ghost"}}"#);
    response.assert_cache_control("no-store");
}

#[tokio::test(flavor = "multi_thread")]
async fn get_returns_not_found_when_explicit_signup_is_disabled() {
    let (_, anon) = TestApp::init()
        .with_config(|config| config.features.explicit_signup_enabled = false)
        .empty()
        .await;

    let response = anon.get::<Value>("/api/private/session/signup").await;
    assert_snapshot!(response.status(), @"404 Not Found");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Not Found"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn get_rejects_signed_in_user() {
    let (_, _, user) = TestApp::init().with_user().await;

    let response = user.get::<Value>("/api/private/session/signup").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"You are already signed in."}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn get_rejects_missing_signup() {
    let (_, anon) = TestApp::init().empty().await;

    let response = anon.get::<Value>("/api/private/session/signup").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Your signup session is missing or has expired. Please authenticate with GitHub again."}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn get_rejects_and_clears_malformed_signup() {
    let (app, anon) = TestApp::init().empty().await;
    let values = HashMap::from([(PENDING_SIGNUP_KEY.into(), "not-json".into())]);
    let cookie = encode_session_header(app.as_inner().session_key(), values);
    let mut request = anon.request_builder(Method::GET, "/api/private/session/signup");
    request.header(header::COOKIE, &cookie);

    let response = anon.run::<Value>(request).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Your signup session is missing or has expired. Please authenticate with GitHub again."}]}"#);

    let set_cookie = response.headers().get(header::SET_COOKIE).unwrap();
    let cookie = set_cookie.to_str().unwrap();
    let cookie = cookie.split(';').next().unwrap();
    let mut request = anon.request_builder(Method::GET, "/api/private/session/signup");
    request.header(header::COOKIE, cookie);

    let response = anon.run::<Value>(request).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert!(response.headers().get(header::SET_COOKIE).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn get_rejects_and_clears_expired_signup() {
    let (app, anon) = TestApp::init().empty().await;
    let created_at = Utc::now() - Duration::minutes(31);
    let cookie = pending_signup_header(&app, created_at);
    let mut request = anon.request_builder(Method::GET, "/api/private/session/signup");
    request.header(header::COOKIE, &cookie);

    let response = anon.run::<Value>(request).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.json(), @r#"{"errors":[{"detail":"Your signup session is missing or has expired. Please authenticate with GitHub again."}]}"#);

    let set_cookie = response.headers().get(header::SET_COOKIE).unwrap();
    let cookie = set_cookie.to_str().unwrap();
    let cookie = cookie.split(';').next().unwrap();
    let mut request = anon.request_builder(Method::GET, "/api/private/session/signup");
    request.header(header::COOKIE, cookie);

    let response = anon.run::<Value>(request).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert!(response.headers().get(header::SET_COOKIE).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_clears_pending_signup_and_leaves_user_logged_out() {
    let (app, anon) = TestApp::init().empty().await;
    let cookie = pending_signup_header(&app, Utc::now());
    let mut request = anon.request_builder(Method::DELETE, "/api/private/session/signup");
    request.header(header::COOKIE, &cookie);

    let response = anon.run::<Value>(request).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.json(), @r#"{"ok":true}"#);

    let set_cookie = response.headers().get(header::SET_COOKIE).unwrap();
    let cookie = set_cookie.to_str().unwrap();
    let cookie = cookie.split(';').next().unwrap();
    let mut request = anon.request_builder(Method::GET, "/api/private/session/signup");
    request.header(header::COOKIE, cookie);

    let response = anon.run::<Value>(request).await;
    assert_snapshot!(response.status(), @"400 Bad Request");

    let mut request = anon.request_builder(Method::GET, "/api/v1/me");
    request.header(header::COOKIE, cookie);

    let response = anon.run::<Value>(request).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_without_pending_signup_returns_ok_when_explicit_signup_is_disabled() {
    let (_, anon) = TestApp::init()
        .with_config(|config| config.features.explicit_signup_enabled = false)
        .empty()
        .await;

    let request = anon.request_builder(Method::DELETE, "/api/private/session/signup");
    let response = anon.run::<Value>(request).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.json(), @r#"{"ok":true}"#);
}
