use conduit::{Handler, Method};

use cargo_registry::Model;
use cargo_registry::krate::EncodableCrate;
use cargo_registry::user::{User, NewUser, EncodableUser};
use cargo_registry::version::EncodableVersion;

#[derive(RustcDecodable)]
struct AuthResponse {
    url: String,
    state: String,
}
#[derive(RustcDecodable)]
pub struct UserShowResponse {
    pub user: EncodableUser,
}

#[test]
fn auth_gives_a_token() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/authorize_url");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: AuthResponse = ::json(&mut response);
    assert!(json.url.contains(&json.state));
}

#[test]
fn access_token_needs_data() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/authorize");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: ::Bad = ::json(&mut response);
    assert!(json.errors[0].detail.contains("invalid state"));
}

#[test]
fn user_insert() {
    let (_b, app, _middle) = ::app();
    let conn = t!(app.database.get());
    let tx = t!(conn.transaction());

    let user = t!(User::find_or_insert(&tx, 1, "foo", None, None, None, "bar"));
    assert_eq!(t!(User::find(&tx, user.id)), user);

    assert_eq!(
        t!(User::find_or_insert(&tx, 1, "foo", None, None, None, "bar")),
        user
    );
    let user2 = t!(User::find_or_insert(&tx, 1, "foo", None, None, None, "baz"));
    assert!(user != user2);
    assert_eq!(user.id, user2.id);
    assert_eq!(user2.gh_access_token, "baz");

    let user3 = t!(User::find_or_insert(&tx, 1, "bar", None, None, None, "baz"));
    assert!(user != user3);
    assert_eq!(user.id, user3.id);
    assert_eq!(user3.gh_login, "bar");
}

#[test]
fn me() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/me");
    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.0, 403);

    let user = ::mock_user(&mut req, ::user("foo"));

    let mut response = ok_resp!(middle.call(&mut req));
    let json: UserShowResponse = ::json(&mut response);

    assert_eq!(json.user.email, user.email);
}

#[test]
fn show() {
    let (_b, app, middle) = ::app();
    {
        let conn = t!(app.diesel_database.get());

        t!(NewUser::new(1, "foo", Some("foo@bar.com"), None, None, "bar").create_or_update(&conn));
        t!(NewUser::new(2, "bar", Some("bar@baz.com"), None, None, "bar").create_or_update(&conn));
    }

    let mut req = ::req(app.clone(), Method::Get, "/api/v1/users/foo");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: UserShowResponse = ::json(&mut response);
    assert_eq!(Some("foo@bar.com".into()), json.user.email);
    assert_eq!("foo", json.user.login);

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/users/bar")));
    let json: UserShowResponse = ::json(&mut response);
    assert_eq!(Some("bar@baz.com".into()), json.user.email);
    assert_eq!("bar", json.user.login);
    assert_eq!(Some("https://github.com/bar".into()), json.user.url);
}

#[test]
fn crates_by_user_id() {
    let (_b, app, middle) = ::app();
    let u;
    {
        let conn = app.diesel_database.get().unwrap();
        u = ::new_user("foo").create_or_update(&conn).unwrap();
        ::CrateBuilder::new("foo_my_packages", u.id).expect_build(&conn);
    }

    let mut req = ::req(app, Method::Get, "/api/v1/crates");
    req.with_query(&format!("user_id={}", u.id));
    let mut response = ok_resp!(middle.call(&mut req));

    #[derive(RustcDecodable)]
    struct Response {
        crates: Vec<EncodableCrate>,
    }
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
    #[derive(RustcDecodable)]
    struct Meta {
        more: bool,
    }

    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/");
    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);

        ::CrateBuilder::new("foo_fighters", user.id)
            .version("1.0.0")
            .expect_build(&conn);

        ::CrateBuilder::new("bar_fighters", user.id)
            .version("1.0.0")
            .expect_build(&conn);
    }

    let mut response = ok_resp!(middle.call(
        req.with_path("/me/updates").with_method(Method::Get),
    ));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.versions.len(), 0);
    assert_eq!(r.meta.more, false);

    ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/foo_fighters/follow")
                .with_method(Method::Put),
        )
    );
    ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/bar_fighters/follow")
                .with_method(Method::Put),
        )
    );

    let mut response = ok_resp!(middle.call(
        req.with_path("/me/updates").with_method(Method::Get),
    ));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.versions.len(), 2);
    assert_eq!(r.meta.more, false);

    let mut response = ok_resp!(
        middle.call(
            req.with_path("/me/updates")
                .with_method(Method::Get)
                .with_query("per_page=1"),
        )
    );
    let r = ::json::<R>(&mut response);
    assert_eq!(r.versions.len(), 1);
    assert_eq!(r.meta.more, true);

    ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/bar_fighters/follow")
                .with_method(Method::Delete),
        )
    );
    let mut response = ok_resp!(
        middle.call(
            req.with_path("/me/updates")
                .with_method(Method::Get)
                .with_query("page=2&per_page=1"),
        )
    );
    let r = ::json::<R>(&mut response);
    assert_eq!(r.versions.len(), 0);
    assert_eq!(r.meta.more, false);

    bad_resp!(middle.call(req.with_query("page=0")));
}
