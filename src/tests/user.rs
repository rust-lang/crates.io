use std::error::Error;

use conduit::{Handler, Request, Response, Method};
use conduit_middleware::Middleware;
use conduit_test::MockRequest;

use cargo_registry::krate::EncodableCrate;
use cargo_registry::user::{User, EncodableUser};
use cargo_registry::db::RequestTransaction;
use cargo_registry::version::EncodableVersion;

#[derive(RustcDecodable)]
struct AuthResponse { url: String, state: String }
#[derive(RustcDecodable)]
struct MeResponse { user: EncodableUser, api_token: String }

#[test]
fn auth_gives_a_token() {
    let (_b, _app, middle) = ::app();
    let mut req = MockRequest::new(Method::Get, "/authorize_url");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: AuthResponse = ::json(&mut response);
    assert!(json.url.contains(&json.state));
}

#[test]
fn access_token_needs_data() {
    let (_b, _app, middle) = ::app();
    let mut req = MockRequest::new(Method::Get, "/authorize");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: ::Bad = ::json(&mut response);
    assert!(json.errors[0].detail.contains("invalid state"));
}

#[test]
fn user_insert() {
    let (_b, app, _middle) = ::app();
    let conn = t!(app.database.get());
    let tx = t!(conn.transaction());

    let user = t!(User::find_or_insert(&tx, "foo", None, None, None, "bar", "baz"));
    assert_eq!(t!(User::find_by_api_token(&tx, "baz")), user);
    assert_eq!(t!(User::find(&tx, user.id)), user);

    assert_eq!(t!(User::find_or_insert(&tx, "foo", None, None, None,
                                       "bar", "api")), user);
    let user2 = t!(User::find_or_insert(&tx, "foo", None, None, None,
                                        "baz", "api"));
    assert!(user != user2);
    assert_eq!(user2.gh_access_token, "baz");
}

#[test]
fn me() {
    let (_b, _app, mut middle) = ::app();
    let mut req = MockRequest::new(Method::Get, "/me");
    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.0, 403);

    let user = ::user("foo");
    middle.add(::middleware::MockUser(user.clone()));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: MeResponse = ::json(&mut response);
    assert_eq!(json.user.email, user.email);
    assert_eq!(json.api_token, user.api_token);
}

#[test]
fn reset_token() {
    struct ResetTokenTest;

    let (_b, _app, mut middle) = ::app();
    middle.add(ResetTokenTest);
    let mut req = MockRequest::new(Method::Put, "/me/reset_token");
    ok_resp!(middle.call(&mut req));

    impl Middleware for ResetTokenTest {
        fn before(&self, req: &mut Request) -> Result<(), Box<Error+Send>> {
            let user = User::find_or_insert(req.tx().unwrap(), "foo", None,
                                            None, None, "bar", "baz").unwrap();
            req.mut_extensions().insert(user);
            Ok(())
        }

        fn after(&self, req: &mut Request,
                 response: Result<Response, Box<Error+Send>>)
                 -> Result<Response, Box<Error+Send>> {
            let user = req.extensions().find::<User>().unwrap();
            let u2 = User::find(req.tx().unwrap(), user.id).unwrap();
            assert!(u2.api_token != user.api_token);
            response
        }
    }
}

#[test]
fn my_packages() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/crates");
    let u = ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo"));
    req.with_query(&format!("user_id={}", u.id));
    let mut response = ok_resp!(middle.call(&mut req));

    #[derive(RustcDecodable)]
    struct Response { crates: Vec<EncodableCrate> }
    let response: Response = ::json(&mut response);
    assert_eq!(response.crates.len(), 1);
}

#[test]
fn following() {
    #[derive(RustcDecodable)]
    struct R {
        versions: Vec<EncodableVersion>,
        meta: Meta,
    }
    #[derive(RustcDecodable)] struct Meta { more: bool }

    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/");
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo"));
    ::mock_crate(&mut req, ::krate("bar"));

    let mut response = ok_resp!(middle.call(req.with_path("/me/updates")
                                               .with_method(Method::Get)));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.versions.len(), 0);
    assert_eq!(r.meta.more, false);

    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo/follow")
                            .with_method(Method::Put)));
    ok_resp!(middle.call(req.with_path("/api/v1/crates/bar/follow")
                            .with_method(Method::Put)));

    let mut response = ok_resp!(middle.call(req.with_path("/me/updates")
                                               .with_method(Method::Get)));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.versions.len(), 2);
    assert_eq!(r.meta.more, false);

    let mut response = ok_resp!(middle.call(req.with_path("/me/updates")
                                               .with_method(Method::Get)
                                               .with_query("per_page=1")));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.versions.len(), 1);
    assert_eq!(r.meta.more, true);

    ok_resp!(middle.call(req.with_path("/api/v1/crates/bar/follow")
                            .with_method(Method::Delete)));
    let mut response = ok_resp!(middle.call(req.with_path("/me/updates")
                                               .with_method(Method::Get)
                                               .with_query("page=2&per_page=1")));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.versions.len(), 0);
    assert_eq!(r.meta.more, false);
}
