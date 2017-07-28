use std::sync::atomic::Ordering;

use conduit::{Handler, Method};

use cargo_registry::Model;
use cargo_registry::token::ApiToken;
use cargo_registry::krate::EncodableCrate;
use cargo_registry::user::{User, NewUser, EncodablePrivateUser};
use cargo_registry::version::EncodableVersion;

use diesel::prelude::*;

#[derive(Deserialize)]
struct AuthResponse {
    url: String,
    state: String,
}

#[derive(Deserialize)]
pub struct UserShowResponse {
    pub user: EncodablePrivateUser,
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
    let mut req = ::req(app.clone(), Method::Get, "/me");
    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.0, 403);

    // with GET /me update gives 404 response
    // let user = ::mock_user(&mut req, ::user("foo"));
    let user = {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        user
    };
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
    // Emails should be None as when on the user/:user_id page, a user's email should
    // not be accessible in order to keep private.
    assert_eq!(None, json.user.email);
    assert_eq!("foo", json.user.login);

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/users/bar")));
    let json: UserShowResponse = ::json(&mut response);
    assert_eq!(None, json.user.email);
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

    #[derive(Deserialize)]
    struct Response {
        crates: Vec<EncodableCrate>,
    }
    let response: Response = ::json(&mut response);
    assert_eq!(response.crates.len(), 1);
}

#[test]
fn following() {
    #[derive(Deserialize)]
    struct R {
        versions: Vec<EncodableVersion>,
        meta: Meta,
    }
    #[derive(Deserialize)]
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
            .version(::VersionBuilder::new("1.0.0"))
            .expect_build(&conn);

        ::CrateBuilder::new("bar_fighters", user.id)
            .version(::VersionBuilder::new("1.0.0"))
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

#[test]
fn user_total_downloads() {
    use diesel::update;

    let (_b, app, middle) = ::app();
    let u;
    {
        let conn = app.diesel_database.get().unwrap();

        u = ::new_user("foo").create_or_update(&conn).unwrap();

        let mut krate = ::CrateBuilder::new("foo_krate1", u.id).expect_build(&conn);
        krate.downloads = 10;
        update(&krate).set(&krate).execute(&*conn).unwrap();

        let mut krate2 = ::CrateBuilder::new("foo_krate2", u.id).expect_build(&conn);
        krate2.downloads = 20;
        update(&krate2).set(&krate2).execute(&*conn).unwrap();

        let another_user = ::new_user("bar").create_or_update(&conn).unwrap();

        let mut another_krate = ::CrateBuilder::new("bar_krate1", another_user.id)
            .expect_build(&conn);
        another_krate.downloads = 2;
        update(&another_krate)
            .set(&another_krate)
            .execute(&*conn)
            .unwrap();
    }

    let mut req = ::req(app, Method::Get, &format!("/api/v1/users/{}/stats", u.id));
    let mut response = ok_resp!(middle.call(&mut req));

    #[derive(Deserialize)]
    struct Response {
        total_downloads: i64,
    }
    let response: Response = ::json(&mut response);
    assert_eq!(response.total_downloads, 30);
    assert!(response.total_downloads != 32);
}

#[test]
fn user_total_downloads_no_crates() {
    let (_b, app, middle) = ::app();
    let u;
    {
        let conn = app.diesel_database.get().unwrap();

        u = ::new_user("foo").create_or_update(&conn).unwrap();
    }

    let mut req = ::req(app, Method::Get, &format!("/api/v1/users/{}/stats", u.id));
    let mut response = ok_resp!(middle.call(&mut req));

    #[derive(Deserialize)]
    struct Response {
        total_downloads: i64,
    }
    let response: Response = ::json(&mut response);
    assert_eq!(response.total_downloads, 0);
}

#[test]
fn updating_existing_user_doesnt_change_api_token() {
    let (_b, app, _middle) = ::app();
    let conn = t!(app.diesel_database.get());

    let gh_user_id = ::NEXT_ID.fetch_add(1, Ordering::SeqCst) as i32;

    let original_user =
        t!(NewUser::new(gh_user_id, "foo", None, None, None, "foo_token").create_or_update(&conn));
    let token = t!(ApiToken::insert(&conn, original_user.id, "foo"));

    t!(NewUser::new(gh_user_id, "bar", None, None, None, "bar_token").create_or_update(&conn));
    let user = t!(User::find_by_api_token(&conn, &token.token));

    assert_eq!("bar", user.gh_login);
    assert_eq!("bar_token", user.gh_access_token);
}

/*  Email GitHub private overwrite bug
    Please find a better description, that is not english
*/
#[test]
fn test_github_login_does_not_overwrite_email() {
    #[derive(Deserialize)]
    struct R {
        user: EncodablePrivateUser,
    }

    #[derive(Deserialize)]
    struct S {
        ok: bool,
    }

    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/me");
    let user = {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user_with_id("apricot", 1).create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        user
    };

    let mut response = ok_resp!(middle.call(req.with_path("/me").with_method(Method::Get)));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email, None);
    assert_eq!(r.user.login, "apricot");

    let body = r#"{"user":{"email":"apricot@apricots.apricot","name":"Apricot Apricoto","login":"apricot","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/apricot","kind":null}}"#;
    let mut response = ok_resp!(
        middle.call(
            req.with_path(&format!("/api/v1/users/{}", user.id))
                .with_method(Method::Put)
                .with_body(body.as_bytes())
        )
    );
    assert!(::json::<S>(&mut response).ok);

    ::logout(&mut req);
    let user = {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user_with_id("apricot", 1).create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        user
    };

    let mut response = ok_resp!(middle.call(req.with_path("/me").with_method(Method::Get)));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email.unwrap(), "apricot@apricots.apricot");
    assert_eq!(r.user.login, "apricot");
}

/*  Make sure that what is passed into the database is
    also what is extracted out of the database
*/
#[test]
fn test_email_get_and_put() {
    #[derive(Deserialize)]
    struct R {
        user: EncodablePrivateUser,
    }

    #[derive(Deserialize)]
    struct S {
        ok: bool,
    }

    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/me");
    let user = {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("mango").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        user
    };

    let mut response = ok_resp!(middle.call(req.with_path("/me").with_method(Method::Get)));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email, None);
    assert_eq!(r.user.login, "mango");

    let body = r#"{"user":{"email":"mango@mangos.mango","name":"Mango McMangoface","login":"mango","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/mango","kind":null}}"#;
    let mut response = ok_resp!(
        middle.call(
            req.with_path(&format!("/api/v1/users/{}", user.id))
                .with_method(Method::Put)
                .with_body(body.as_bytes())
        )
    );
    assert!(::json::<S>(&mut response).ok);

    let mut response = ok_resp!(middle.call(req.with_path("/me").with_method(Method::Get)));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email.unwrap(), "mango@mangos.mango");
    assert_eq!(r.user.login, "mango");
}

/*  Make sure that empty strings will error and are
    not added to the database
    Tests for empty string and none. unlikely this
    would ever occur but might as well check it
*/
#[test]
fn test_empty_email_not_added() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/me");
    let user = {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("papaya").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        user
    };

    let body = r#"{"user":{"email":"","name":"Papayo Papaya","login":"papaya","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/papaya","kind":null}}"#;
    let json = bad_resp!(
        middle.call(
            req.with_path(&format!("/api/v1/users/{}", user.id))
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );

    assert!(json.errors[0].detail.contains("empty email rejected"), "{:?}", json.errors);

    let body = r#"{"user":{"email":null,"name":"Papayo Papaya","login":"papaya","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/papaya","kind":null}}"#;
    let json = bad_resp!(
        middle.call(
            req.with_path(&format!("/api/v1/users/{}", user.id))
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );

    assert!(json.errors[0].detail.contains("empty email rejected"), "{:?}", json.errors);
}

/*  A user cannot change the email of another user
    Two users in database, one signed in, the other
    not signed in, from one that is not signed in try to
    change signed in's email

*/
#[test]
fn test_this_user_cannot_change_that_user_email() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/me");

    let not_signed_in_user = {
        let conn = app.diesel_database.get().unwrap();
        let signed_user = ::new_user("pineapple").create_or_update(&conn).unwrap();
        let unsigned_user = ::new_user("coconut").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &signed_user);
        unsigned_user
    };

    let body = r#"{"user":{"email":"pineapple@pineapples.pineapple","name":"Pine Apple","login":"pineapple","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/pineapple","kind":null}}"#;

    let json = bad_resp!(
        middle.call(
            req.with_path(&format!("/api/v1/users/{}", not_signed_in_user.id))
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );

    assert!(json.errors[0].detail.contains("current user does not match requested user"), "{:?}", json.errors);

}
