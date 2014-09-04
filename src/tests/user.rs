use conduit_test::MockRequest;
use conduit::{mod, Handler};

use cargo_registry::user::{User, EncodableUser};

#[deriving(Decodable)]
struct AuthResponse { url: String, state: String }
#[deriving(Decodable)]
struct TokenResponse { ok: bool, error: Option<String> }
#[deriving(Decodable)]
struct MeResponse { ok: bool, error: Option<EncodableUser> }

#[test]
fn auth_gives_a_token() {
    let middle = ::middleware();
    let mut req = MockRequest::new(conduit::Get, "/authorize_url");
    let mut response = t_resp!(middle.call(&mut req));
    let json: AuthResponse = ::json(&mut response);
    assert!(json.url.as_slice().contains(json.state.as_slice()));
}

#[test]
fn access_token_needs_data() {
    let middle = ::middleware();
    let mut req = MockRequest::new(conduit::Get, "/authorize");
    let mut response = t_resp!(middle.call(&mut req));
    let json: TokenResponse = ::json(&mut response);
    assert!(!json.ok);
    assert!(json.error.is_some());
    assert!(json.error.unwrap().as_slice().contains("invalid state"));
}

#[test]
fn user_insert() {
    let app = ::app();
    let conn = t!(app.database.get());
    let tx = t!(conn.transaction());

    let user = t!(User::find_or_insert(&tx, "foo", "bar"));
    assert_eq!(t!(User::find_by_api_token(&tx, user.api_token.as_slice())),
               user);
    assert_eq!(t!(User::find(&tx, user.id)), user);

    assert_eq!(t!(User::find_or_insert(&tx, "foo", "bar")), user);
    let user2 = t!(User::find_or_insert(&tx, "foo", "baz"));
    assert!(user != user2);
    assert_eq!(user2.gh_access_token.as_slice(), "baz");
}

#[test]
fn me() {
    let middle = ::middleware();
    let mut req = MockRequest::new(conduit::Get, "/me");
    let response = t!(middle.call(&mut req).map_err(|e| (&*e).to_string()));
    assert_eq!(response.status.val0(), 403);
}
