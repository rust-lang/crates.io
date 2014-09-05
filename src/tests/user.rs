use std::fmt::Show;

use conduit::{mod, Handler, Request, Response};
use conduit_middleware::Middleware;
use conduit_test::MockRequest;

use cargo_registry::user::{User, EncodableUser};
use cargo_registry::db::RequestTransaction;

#[deriving(Decodable)]
struct AuthResponse { url: String, state: String }
#[deriving(Decodable)]
struct TokenResponse { ok: bool, error: Option<String> }
#[deriving(Decodable)]
struct MeResponse { ok: bool, user: EncodableUser }

#[test]
fn auth_gives_a_token() {
    let middle = ::middleware();
    let mut req = MockRequest::new(conduit::Get, "/authorize_url");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: AuthResponse = ::json(&mut response);
    assert!(json.url.as_slice().contains(json.state.as_slice()));
}

#[test]
fn access_token_needs_data() {
    let middle = ::middleware();
    let mut req = MockRequest::new(conduit::Get, "/authorize");
    let mut response = ok_resp!(middle.call(&mut req));
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

    let user = t!(User::find_or_insert(&tx, "foo", "bar", "baz"));
    assert_eq!(t!(User::find_by_api_token(&tx, "baz")), user);
    assert_eq!(t!(User::find(&tx, user.id)), user);

    assert_eq!(t!(User::find_or_insert(&tx, "foo", "bar", "api")), user);
    let user2 = t!(User::find_or_insert(&tx, "foo", "baz", "api"));
    assert!(user != user2);
    assert_eq!(user2.gh_access_token.as_slice(), "baz");
}

#[test]
fn me() {
    let mut middle = ::middleware();
    let mut req = MockRequest::new(conduit::Get, "/me");
    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.val0(), 403);

    let user = ::user();
    middle.add(::middleware::MockUser(user.clone()));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: MeResponse = ::json(&mut response);
    assert!(json.ok);
    assert_eq!(json.user.email, user.email);
    assert_eq!(json.user.api_token, user.api_token);
    assert_eq!(json.user.id, user.id);
}

#[test]
fn reset_token() {
    struct ResetTokenTest;

    let mut middle = ::middleware();
    middle.add(ResetTokenTest);
    let mut req = MockRequest::new(conduit::Put, "/me/reset_token");
    ok_resp!(middle.call(&mut req));

    impl Middleware for ResetTokenTest {
        fn before(&self, req: &mut Request) -> Result<(), Box<Show + 'static>> {
            let user = User::find_or_insert(req.tx().unwrap(), "foo",
                                            "bar", "baz").unwrap();
            req.mut_extensions().insert(user);
            Ok(())
        }

        fn after(&self, req: &mut Request,
                 response: Result<Response, Box<Show + 'static>>)
                 -> Result<Response, Box<Show + 'static>> {
            let user = req.extensions().find::<User>().unwrap();
            let u2 = User::find(req.tx().unwrap(), user.id).unwrap();
            assert!(u2.api_token != user.api_token);
            response
        }
    }
}
