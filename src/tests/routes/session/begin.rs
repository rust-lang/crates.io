use crate::tests::util::{RequestHelper, TestApp};

#[derive(Deserialize)]
struct AuthResponse {
    url: String,
    state: String,
}

#[tokio::test(flavor = "multi_thread")]
async fn auth_gives_a_token() {
    let (_, anon) = TestApp::init().empty().await;
    let json: AuthResponse = anon.get("/api/private/session/begin").await.good();
    assert!(json.url.contains(&json.state));
}
