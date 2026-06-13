use crate::util::{RequestHelper, TestApp};
use insta::assert_snapshot;
use oauth2::ClientId;
use serde::Deserialize;
use url::Url;

#[derive(Deserialize)]
struct AuthResponse {
    url: String,
}

#[tokio::test(flavor = "multi_thread")]
async fn post_gives_a_token() {
    let (_, anon) = TestApp::init()
        .with_config(|config| config.gh_client_id = ClientId::new("test-client-id".into()))
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
