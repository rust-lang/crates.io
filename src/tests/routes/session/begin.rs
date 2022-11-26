use crate::util::{RequestHelper, TestApp};

#[derive(Deserialize)]
struct AuthResponse {
    url: String,
    state: String,
}

#[test]
fn auth_gives_a_token() {
    let (_, anon) = TestApp::init().empty();
    let json: AuthResponse = anon.get("/api/private/session/begin").good();
    assert!(json.url.contains(&json.state));
}
