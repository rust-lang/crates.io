use std::sync::atomic::Ordering;

use conduit::{Handler, Method};

use cargo_registry::token::ApiToken;
use cargo_registry::krate::EncodableCrate;
use cargo_registry::user::{Email, EncodablePrivateUser, EncodablePublicUser, NewUser, User};
use cargo_registry::version::EncodableVersion;

use diesel::prelude::*;

#[derive(Deserialize)]
struct AuthResponse {
    url: String,
    state: String,
}

#[derive(Deserialize)]
pub struct UserShowPublicResponse {
    pub user: EncodablePublicUser,
}

#[derive(Deserialize)]
pub struct UserShowPrivateResponse {
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
fn me() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/me");
    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.0, 403);

    let user = ::sign_in(&mut req, &app);

    let mut response = ok_resp!(middle.call(&mut req));
    let json: UserShowPrivateResponse = ::json(&mut response);

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
    let json: UserShowPublicResponse = ::json(&mut response);
    assert_eq!("foo", json.user.login);

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/users/bar")));
    let json: UserShowPublicResponse = ::json(&mut response);
    assert_eq!("bar", json.user.login);
    assert_eq!(Some("https://github.com/bar".into()), json.user.url);
}

#[test]
fn show_latest_user_case_insensitively() {
    let (_b, app, middle) = ::app();
    {
        let conn = t!(app.diesel_database.get());

        // Please do not delete or modify the setup of this test in order to get it to pass.
        // This setup mimics how GitHub works. If someone abandons a GitHub account, the username is
        // available for anyone to take. We need to support having multiple user accounts
        // with the same gh_login in crates.io. `gh_id` is stable across renames, so that field
        // should be used for uniquely identifying GitHub accounts whenever possible. For the
        // crates.io/user/:username pages, the best we can do is show the last crates.io account
        // created with that username.
        t!(
            NewUser::new(
                1,
                "foobar",
                Some("foo@bar.com"),
                Some("I was first then deleted my github account"),
                None,
                "bar"
            ).create_or_update(&conn)
        );
        t!(
            NewUser::new(
                2,
                "FOOBAR",
                Some("later-foo@bar.com"),
                Some("I was second, I took the foobar username on github"),
                None,
                "bar"
            ).create_or_update(&conn)
        );
    }
    let mut req = ::req(app.clone(), Method::Get, "api/v1/users/fOObAr");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: UserShowPublicResponse = ::json(&mut response);
    assert_eq!(
        "I was second, I took the foobar username on github",
        json.user.name.unwrap()
    );
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
fn crates_by_user_id_not_including_deleted_owners() {
    let (_b, app, middle) = ::app();
    let u;
    {
        let conn = app.diesel_database.get().unwrap();
        u = ::new_user("foo").create_or_update(&conn).unwrap();
        let krate = ::CrateBuilder::new("foo_my_packages", u.id).expect_build(&conn);
        krate.owner_remove(&app, &conn, &u, "foo").unwrap();
    }

    let mut req = ::req(app, Method::Get, "/api/v1/crates");
    req.with_query(&format!("user_id={}", u.id));
    let mut response = ok_resp!(middle.call(&mut req));

    #[derive(Deserialize)]
    struct Response {
        crates: Vec<EncodableCrate>,
    }
    let response: Response = ::json(&mut response);
    assert_eq!(response.crates.len(), 0);
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

    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/me/updates",)
                .with_method(Method::Get,),
        )
    );
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

    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/me/updates",)
                .with_method(Method::Get,),
        )
    );
    let r = ::json::<R>(&mut response);
    assert_eq!(r.versions.len(), 2);
    assert_eq!(r.meta.more, false);

    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/me/updates")
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
            req.with_path("/api/v1/me/updates")
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

        let mut another_krate =
            ::CrateBuilder::new("bar_krate1", another_user.id).expect_build(&conn);
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

/*  Given a GitHub user, check that if the user logs in,
    updates their email, logs out, then logs back in, the
    email they added to crates.io will not be overwritten
    by the information sent by GitHub.

    This bug is problematic if the user's email preferences
    are set to private on GitHub, as GitHub will always
    send none as the email and we will end up inadvertenly
    deleting their email when they sign back in.
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
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/me");
    let user = {
        let conn = app.diesel_database.get().unwrap();
        let user = NewUser {
            gh_id: 1,
            ..::new_user("apricot")
        };

        let user = user.create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        user
    };

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email, None);
    assert_eq!(r.user.login, "apricot");

    let body = r#"{"user":{"email":"apricot@apricots.apricot","name":"Apricot Apricoto","login":"apricot","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/apricot","kind":null}}"#;
    let mut response = ok_resp!(
        middle.call(
            req.with_path(&format!("/api/v1/users/{}", user.id))
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(::json::<S>(&mut response).ok);

    ::logout(&mut req);

    {
        let conn = app.diesel_database.get().unwrap();
        let user = NewUser {
            gh_id: 1,
            ..::new_user("apricot")
        };

        let user = user.create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
    }

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email.unwrap(), "apricot@apricots.apricot");
    assert_eq!(r.user.login, "apricot");
}

/*  Given a crates.io user, check that the user's email can be
    updated in the database (PUT /user/:user_id), then check
    that the updated email is sent back to the user (GET /me).
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
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/me");
    let user = {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("mango").create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        user
    };

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email, None);
    assert_eq!(r.user.login, "mango");

    let body = r#"{"user":{"email":"mango@mangos.mango","name":"Mango McMangoface","login":"mango","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/mango","kind":null}}"#;
    let mut response = ok_resp!(
        middle.call(
            req.with_path(&format!("/api/v1/users/{}", user.id))
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(::json::<S>(&mut response).ok);

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email.unwrap(), "mango@mangos.mango");
    assert_eq!(r.user.login, "mango");
    assert!(!r.user.email_verified);
    assert!(r.user.email_verification_sent);
}

/*  Given a crates.io user, check to make sure that the user
    cannot add to the database an empty string or null as
    their email. If an attempt is made, update_user.rs will
    return an error indicating that an empty email cannot be
    added.

    This is checked on the frontend already, but I'd like to
    make sure that a user cannot get around that and delete
    their email by adding an empty string.
*/
#[test]
fn test_empty_email_not_added() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/me");
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

    assert!(
        json.errors[0].detail.contains("empty email rejected"),
        "{:?}",
        json.errors
    );

    let body = r#"{"user":{"email":null,"name":"Papayo Papaya","login":"papaya","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/papaya","kind":null}}"#;
    let json = bad_resp!(
        middle.call(
            req.with_path(&format!("/api/v1/users/{}", user.id))
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );

    assert!(
        json.errors[0].detail.contains("empty email rejected"),
        "{:?}",
        json.errors
    );
}

/*  Given two users, one signed in and the other not signed in,
    check to make sure that the not signed in user cannot edit
    the email of the signed in user, or vice-versa.

    If an attempt is made, update_user.rs will return an error
    indicating that the current user does not match the
    requested user.
*/
#[test]
fn test_this_user_cannot_change_that_user_email() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/me");

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

    assert!(
        json.errors[0]
            .detail
            .contains("current user does not match requested user",),
        "{:?}",
        json.errors
    );
}

/* Given a new user, test that if they sign in with
   one email, change their email on GitHub, then
   sign in again, that the email will remain
   consistent with the original email used on
   GitHub.
*/
#[test]
fn test_insert_into_email_table() {
    #[derive(Deserialize)]
    struct R {
        user: EncodablePrivateUser,
    }

    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/me");
    {
        let conn = app.diesel_database.get().unwrap();
        let user = NewUser {
            gh_id: 1,
            email: Some("apple@example.com"),
            ..::new_user("apple")
        };

        let user = user.create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
    }

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email.unwrap(), "apple@example.com");
    assert_eq!(r.user.login, "apple");

    ::logout(&mut req);

    // What if user changes their github user email
    {
        let conn = app.diesel_database.get().unwrap();
        let user = NewUser {
            gh_id: 1,
            email: Some("banana@example.com"),
            ..::new_user("apple")
        };

        let user = user.create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
    }

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email.unwrap(), "apple@example.com");
    assert_eq!(r.user.login, "apple");
}

/* Given a new user, check that when an email is added,
   changed by user on GitHub, changed on crates.io,
   that the email remains consistent with that which
   the user has changed
*/
#[test]
fn test_insert_into_email_table_with_email_change() {
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
        let user = NewUser {
            gh_id: 1,
            email: Some("potato@example.com"),
            ..::new_user("potato")
        };

        let user = user.create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        user
    };

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email.unwrap(), "potato@example.com");
    assert_eq!(r.user.login, "potato");
    assert!(!r.user.email_verified);
    assert!(r.user.email_verification_sent);

    let body = r#"{"user":{"email":"apricot@apricots.apricot","name":"potato","login":"potato","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/potato","kind":null}}"#;
    let mut response = ok_resp!(
        middle.call(
            req.with_path(&format!("/api/v1/users/{}", user.id))
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(::json::<S>(&mut response).ok);

    ::logout(&mut req);

    // What if user changes their github user email
    {
        let conn = app.diesel_database.get().unwrap();
        let user = NewUser {
            gh_id: 1,
            email: Some("banana@example.com"),
            ..::new_user("potato")
        };

        let user = user.create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
    }

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email.unwrap(), "apricot@apricots.apricot");
    assert!(!r.user.email_verified);
    assert!(r.user.email_verification_sent);
    assert_eq!(r.user.login, "potato");
}

/* Given a new user, test that their email can be added
   to the email table and a token for the email is generated
   and added to the token table. When /confirm/:email_token is
   requested, check that the response back is ok, and that
   the email_verified field on user is now set to true.
*/
#[test]
fn test_confirm_user_email() {
    use cargo_registry::schema::emails;

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
        let user = NewUser {
            email: Some("potato2@example.com"),
            ..::new_user("potato")
        };

        let user = user.create_or_update(&conn).unwrap();
        ::sign_in_as(&mut req, &user);
        user
    };

    let email_token = {
        let conn = app.diesel_database.get().unwrap();
        Email::belonging_to(&user)
            .select(emails::token)
            .first::<String>(&*conn)
            .unwrap()
    };

    let mut response = ok_resp!(
        middle.call(
            req.with_path(&format!("/api/v1/confirm/{}", email_token))
                .with_method(Method::Put),
        )
    );
    assert!(::json::<S>(&mut response).ok);

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email.unwrap(), "potato2@example.com");
    assert_eq!(r.user.login, "potato");
    assert!(r.user.email_verified);
    assert!(r.user.email_verification_sent);
}

/* Given a user who existed before we added email confirmation,
   test that `email_verification_sent` is false so that we don't
   make the user think we've sent an email when we haven't.
*/
#[test]
fn test_existing_user_email() {
    use cargo_registry::schema::emails;
    use chrono::NaiveDateTime;
    use diesel::update;

    #[derive(Deserialize)]
    struct R {
        user: EncodablePrivateUser,
    }

    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/me");
    {
        let conn = app.diesel_database.get().unwrap();
        let new_user = NewUser {
            email: Some("potahto@example.com"),
            ..::new_user("potahto")
        };
        let user = new_user.create_or_update(&conn).unwrap();
        update(Email::belonging_to(&user))
            // Users created before we added verification will have
            // `NULL` in the `token_generated_at` column.
            .set(emails::token_generated_at.eq(None::<NaiveDateTime>))
            .execute(&*conn)
            .unwrap();
        ::sign_in_as(&mut req, &user);
    }

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = ::json::<R>(&mut response);
    assert_eq!(r.user.email.unwrap(), "potahto@example.com");
    assert!(!r.user.email_verified);
    assert!(!r.user.email_verification_sent);
}
