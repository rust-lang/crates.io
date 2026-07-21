use crate::util::{MockRequestExt, RequestHelper, TestApp, encode_session_data_header};
use crates_io_database::models::User;
use crates_io_github::GitHubUser;
use http::{Method, header};
use insta::{assert_json_snapshot, assert_snapshot};
use oauth2::ClientId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use url::Url;

#[derive(Deserialize)]
struct AuthResponse {
    url: String,
}

#[tokio::test(flavor = "multi_thread")]
async fn post_gives_a_token() {
    let (_, anon) = TestApp::init()
        .with_config(|config| {
            config.github_oauth.client_id = ClientId::new("test-client-id".into())
        })
        .empty()
        .await;

    let json: AuthResponse = anon.post("/api/private/session/begin", "").await.good();

    let url = Url::parse(&json.url).unwrap();
    let state = url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.into_owned())
        .expect("missing `state` query parameter");

    // The `state` is an oauth2 CSRF token: 16 random bytes encoded as URL-safe
    // base64 without padding, which results in 22 characters.
    assert_eq!(state.len(), 22);
    assert!(state.bytes().all(is_base64));

    let url = json.url.replace(&state, "[STATE]");
    assert_snapshot!(url, @"https://github.com/login/oauth/authorize?response_type=code&client_id=test-client-id&state=[STATE]&scope=read%3Aorg");
}

/// Checks whether `b` is a URL-safe base64 character.
fn is_base64(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

#[derive(Serialize)]
pub struct ConfirmedUserInfo {
    email: Option<String>,
    publish_notifications: Option<bool>,
}

#[tokio::test]
async fn session_confirm_combines_cookie_and_json_for_new_user() {
    let (app, anon) = TestApp::init().empty().await;
    let conn = app.db_conn().await;

    let gh_user = GitHubUser {
        email: Some("public_email@example.com".into()),
        name: Some("My Name".into()),
        login: "github_user".into(),
        id: 12345,
        avatar_url: Some("http://example.com/avatar.png".into()),
    };

    let session_key = app.as_inner().session_key();
    let data = HashMap::from([
        (
            String::from("github_user"),
            serde_json::to_string(&gh_user).unwrap(),
        ),
        (
            String::from("encrypted_token"),
            String::from("0123456789abcdef"),
        ),
    ]);

    let cookie = encode_session_data_header(session_key, &data);

    let user_submitted_json = ConfirmedUserInfo {
        email: Some("cratesio_email@example.com".into()),
        publish_notifications: Some(true),
    };

    let mut request = anon
        .request_builder(Method::POST, "/api/private/session/confirm")
        .with_body(json!(user_submitted_json).to_string().into());
    request.header(header::COOKIE, &cookie);

    let response = anon.run::<()>(request).await;
    assert_snapshot!(response.status(), @"200 OK");
    let json = response.json();
    assert_json_snapshot!(json, {
        ".user.created_at" => "[datetime]",
    });

    let user_id = json
        .get("user")
        .unwrap()
        .get("id")
        .unwrap()
        .as_i64()
        .unwrap() as i32;
    assert_ne!(user_id, -1);

    let user = User::find(&conn, user_id).await.unwrap();
    assert_eq!(user.username, "github_user");
    assert_eq!(user.name.as_ref().unwrap(), "My Name");
    assert_eq!(user.gh_id, 12345);
    assert!(user.publish_notifications);

    let email = user.email(&conn).await.unwrap().unwrap();
    assert_eq!(email, "cratesio_email@example.com");
}
