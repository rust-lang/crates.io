use crate::{
    app,
    builders::{CrateBuilder, VersionBuilder},
    logout, new_user, req, sign_in_as,
    util::RequestHelper,
    OkBool, TestApp,
};
use cargo_registry::{
    models::{Email, NewUser, User},
    views::{EncodablePrivateUser, EncodablePublicUser, EncodableVersion},
};

use conduit::{Handler, Method};
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

#[derive(Deserialize)]
struct UserStats {
    total_downloads: i64,
}

#[test]
fn auth_gives_a_token() {
    let (_, anon) = TestApp::init().empty();
    let json: AuthResponse = anon.get("/authorize_url").good();
    assert!(json.url.contains(&json.state));
}

#[test]
fn access_token_needs_data() {
    let (_, anon) = TestApp::init().empty();
    let json = anon.get::<()>("/authorize").bad_with_status(200); // Change endpoint to 400?
    assert!(json.errors[0].detail.contains("invalid state"));
}

#[test]
fn me() {
    let url = "/api/v1/me";
    let (app, anon) = TestApp::init().empty();
    anon.get(url).assert_forbidden();

    let user = app.db_new_user("foo");
    let json: UserShowPrivateResponse = user.get(url).good();

    assert_eq!(json.user.email, user.as_model().email);
}

#[test]
fn show() {
    let (app, anon, _) = TestApp::init().with_user();
    app.db_new_user("bar");

    let json: UserShowPublicResponse = anon.get("/api/v1/users/foo").good();
    assert_eq!("foo", json.user.login);

    let json: UserShowPublicResponse = anon.get("/api/v1/users/bar").good();
    assert_eq!("bar", json.user.login);
    assert_eq!(Some("https://github.com/bar".into()), json.user.url);
}

#[test]
fn show_latest_user_case_insensitively() {
    let (app, anon) = TestApp::init().empty();

    app.db(|conn| {
        // Please do not delete or modify the setup of this test in order to get it to pass.
        // This setup mimics how GitHub works. If someone abandons a GitHub account, the username is
        // available for anyone to take. We need to support having multiple user accounts
        // with the same gh_login in crates.io. `gh_id` is stable across renames, so that field
        // should be used for uniquely identifying GitHub accounts whenever possible. For the
        // crates.io/user/:username pages, the best we can do is show the last crates.io account
        // created with that username.
        t!(NewUser::new(
            1,
            "foobar",
            Some("foo@bar.com"),
            Some("I was first then deleted my github account"),
            None,
            "bar"
        )
        .create_or_update(conn));
        t!(NewUser::new(
            2,
            "FOOBAR",
            Some("later-foo@bar.com"),
            Some("I was second, I took the foobar username on github"),
            None,
            "bar"
        )
        .create_or_update(conn));
    });

    let json: UserShowPublicResponse = anon.get("api/v1/users/fOObAr").good();
    assert_eq!(
        "I was second, I took the foobar username on github",
        json.user.name.unwrap()
    );
}

#[test]
fn crates_by_user_id() {
    let (app, _, user) = TestApp::init().with_user();
    let id = user.as_model().id;
    app.db(|conn| {
        CrateBuilder::new("foo_my_packages", id).expect_build(conn);
    });

    let response = user.search_by_user_id(id);
    assert_eq!(response.crates.len(), 1);
}

#[test]
fn crates_by_user_id_not_including_deleted_owners() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let krate = CrateBuilder::new("foo_my_packages", user.id).expect_build(conn);
        krate
            .owner_remove(app.as_inner(), conn, user, "foo")
            .unwrap();
    });

    let response = anon.search_by_user_id(user.id);
    assert_eq!(response.crates.len(), 0);
}

#[test]
fn following() {
    use cargo_registry::schema::versions;
    use diesel::update;

    #[derive(Deserialize)]
    struct R {
        versions: Vec<EncodableVersion>,
        meta: Meta,
    }
    #[derive(Deserialize)]
    struct Meta {
        more: bool,
    }

    let (app, _, user) = TestApp::init().with_user();
    let user_model = user.as_model();
    let user_id = user_model.id;
    app.db(|conn| {
        CrateBuilder::new("foo_fighters", user_id)
            .version(VersionBuilder::new("1.0.0"))
            .expect_build(conn);

        // Make foo_fighters's version mimic a version published before we started recording who
        // published versions
        let none: Option<i32> = None;
        update(versions::table)
            .set(versions::published_by.eq(none))
            .execute(conn)
            .unwrap();

        CrateBuilder::new("bar_fighters", user_id)
            .version(VersionBuilder::new("1.0.0"))
            .expect_build(conn);
    });

    let r: R = user.get("/api/v1/me/updates").good();
    assert_eq!(r.versions.len(), 0);
    assert_eq!(r.meta.more, false);

    user.put::<OkBool>("/api/v1/crates/foo_fighters/follow", b"")
        .good();
    user.put::<OkBool>("/api/v1/crates/bar_fighters/follow", b"")
        .good();

    let r: R = user.get("/api/v1/me/updates").good();
    assert_eq!(r.versions.len(), 2);
    assert_eq!(r.meta.more, false);
    let foo_version = r
        .versions
        .iter()
        .find(|v| v.krate == "foo_fighters")
        .unwrap();
    assert!(foo_version.published_by.is_none());
    let bar_version = r
        .versions
        .iter()
        .find(|v| v.krate == "bar_fighters")
        .unwrap();
    assert_eq!(
        bar_version.published_by.as_ref().unwrap().login,
        user_model.gh_login
    );

    let r: R = user
        .get_with_query("/api/v1/me/updates", "per_page=1")
        .good();
    assert_eq!(r.versions.len(), 1);
    assert_eq!(r.meta.more, true);

    user.delete::<OkBool>("/api/v1/crates/bar_fighters/follow")
        .good();
    let r: R = user
        .get_with_query("/api/v1/me/updates", "page=2&per_page=1")
        .good();
    assert_eq!(r.versions.len(), 0);
    assert_eq!(r.meta.more, false);

    user.get_with_query::<()>("/api/v1/me/updates", "page=0")
        .bad_with_status(200); // TODO: Should be 500
}

#[test]
fn user_total_downloads() {
    use diesel::update;

    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    let another_user = app.db_new_user("bar");
    let another_user = another_user.as_model();

    app.db(|conn| {
        let mut krate = CrateBuilder::new("foo_krate1", user.id).expect_build(conn);
        krate.downloads = 10;
        update(&krate).set(&krate).execute(conn).unwrap();

        let mut krate2 = CrateBuilder::new("foo_krate2", user.id).expect_build(conn);
        krate2.downloads = 20;
        update(&krate2).set(&krate2).execute(conn).unwrap();

        let mut another_krate = CrateBuilder::new("bar_krate1", another_user.id).expect_build(conn);
        another_krate.downloads = 2;
        update(&another_krate)
            .set(&another_krate)
            .execute(conn)
            .unwrap();
    });

    let url = format!("/api/v1/users/{}/stats", user.id);
    let stats: UserStats = anon.get(&url).good();
    assert_eq!(stats.total_downloads, 30); // instead of 32
}

#[test]
fn user_total_downloads_no_crates() {
    let (_, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    let url = format!("/api/v1/users/{}/stats", user.id);

    let stats: UserStats = anon.get(&url).good();
    assert_eq!(stats.total_downloads, 0);
}

#[test]
fn updating_existing_user_doesnt_change_api_token() {
    let (app, _, user, token) = TestApp::init().with_token();
    let gh_id = user.as_model().gh_id;
    let token = &token.as_model().token;

    let user = app.db(|conn| {
        // Reuse gh_id but use new gh_login and gh_access_token
        t!(NewUser::new(gh_id, "bar", None, None, None, "bar_token").create_or_update(conn));

        // Use the original API token to find the now updated user
        t!(User::find_by_api_token(conn, token))
    });

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
    let (app, middle) = app();
    let mut req = req(Method::Get, "/api/v1/me");
    let user = {
        let conn = app.diesel_database.get().unwrap();
        let user = new_user("apricot");

        let user = user.create_or_update(&conn).unwrap();
        sign_in_as(&mut req, &user);
        user
    };

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = crate::json::<UserShowPrivateResponse>(&mut response);
    assert_eq!(r.user.email, None);
    assert_eq!(r.user.login, "apricot");

    let body =
        r#"{"user":{"email":"apricot@apricots.apricot","name":"Apricot Apricoto","login":"apricot","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/apricot","kind":null}}"#;
    let mut response = ok_resp!(middle.call(
        req.with_path(&format!("/api/v1/users/{}", user.id))
            .with_method(Method::Put)
            .with_body(body.as_bytes()),
    ));
    assert!(crate::json::<OkBool>(&mut response).ok);

    logout(&mut req);

    {
        let conn = app.diesel_database.get().unwrap();
        let user = NewUser {
            gh_id: user.gh_id,
            ..new_user("apricot")
        };

        let user = user.create_or_update(&conn).unwrap();
        sign_in_as(&mut req, &user);
    }

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = crate::json::<UserShowPrivateResponse>(&mut response);
    assert_eq!(r.user.email.unwrap(), "apricot@apricots.apricot");
    assert_eq!(r.user.login, "apricot");
}

/*  Given a crates.io user, check that the user's email can be
    updated in the database (PUT /user/:user_id), then check
    that the updated email is sent back to the user (GET /me).
*/
#[test]
fn test_email_get_and_put() {
    let (app, middle) = app();
    let mut req = req(Method::Get, "/api/v1/me");
    let user = {
        let conn = app.diesel_database.get().unwrap();
        let user = new_user("mango").create_or_update(&conn).unwrap();
        sign_in_as(&mut req, &user);
        user
    };

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = crate::json::<UserShowPrivateResponse>(&mut response);
    assert_eq!(r.user.email, None);
    assert_eq!(r.user.login, "mango");

    let body =
        r#"{"user":{"email":"mango@mangos.mango","name":"Mango McMangoface","login":"mango","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/mango","kind":null}}"#;
    let mut response = ok_resp!(middle.call(
        req.with_path(&format!("/api/v1/users/{}", user.id))
            .with_method(Method::Put)
            .with_body(body.as_bytes()),
    ));
    assert!(crate::json::<OkBool>(&mut response).ok);

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = crate::json::<UserShowPrivateResponse>(&mut response);
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
    let (app, middle) = app();
    let mut req = req(Method::Get, "/api/v1/me");
    let user = {
        let conn = app.diesel_database.get().unwrap();
        let user = new_user("papaya").create_or_update(&conn).unwrap();
        sign_in_as(&mut req, &user);
        user
    };

    let body =
        r#"{"user":{"email":"","name":"Papayo Papaya","login":"papaya","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/papaya","kind":null}}"#;
    let json = bad_resp!(middle.call(
        req.with_path(&format!("/api/v1/users/{}", user.id))
            .with_method(Method::Put)
            .with_body(body.as_bytes()),
    ));

    assert!(
        json.errors[0].detail.contains("empty email rejected"),
        "{:?}",
        json.errors
    );

    let body =
        r#"{"user":{"email":null,"name":"Papayo Papaya","login":"papaya","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/papaya","kind":null}}"#;
    let json = bad_resp!(middle.call(
        req.with_path(&format!("/api/v1/users/{}", user.id))
            .with_method(Method::Put)
            .with_body(body.as_bytes()),
    ));

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
    let (app, middle) = app();
    let mut req = req(Method::Get, "/api/v1/me");

    let not_signed_in_user = {
        let conn = app.diesel_database.get().unwrap();
        let signed_user = new_user("pineapple").create_or_update(&conn).unwrap();
        let unsigned_user = new_user("coconut").create_or_update(&conn).unwrap();
        sign_in_as(&mut req, &signed_user);
        unsigned_user
    };

    let body =
        r#"{"user":{"email":"pineapple@pineapples.pineapple","name":"Pine Apple","login":"pineapple","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/pineapple","kind":null}}"#;

    let json = bad_resp!(middle.call(
        req.with_path(&format!("/api/v1/users/{}", not_signed_in_user.id))
            .with_method(Method::Put)
            .with_body(body.as_bytes()),
    ));

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
    let (app, middle) = app();
    let mut req = req(Method::Get, "/me");
    let user = {
        let conn = app.diesel_database.get().unwrap();
        let user = NewUser {
            email: Some("apple@example.com"),
            ..new_user("apple")
        };

        let user = user.create_or_update(&conn).unwrap();
        sign_in_as(&mut req, &user);
        user
    };

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = crate::json::<UserShowPrivateResponse>(&mut response);
    assert_eq!(r.user.email.unwrap(), "apple@example.com");
    assert_eq!(r.user.login, "apple");

    logout(&mut req);

    // What if user changes their github user email
    {
        let conn = app.diesel_database.get().unwrap();
        let user = NewUser {
            gh_id: user.gh_id,
            email: Some("banana@example.com"),
            ..new_user("apple")
        };

        let user = user.create_or_update(&conn).unwrap();
        sign_in_as(&mut req, &user);
    }

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = crate::json::<UserShowPrivateResponse>(&mut response);
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
    let (app, middle) = app();
    let mut req = req(Method::Get, "/me");
    let user = {
        let conn = app.diesel_database.get().unwrap();
        let user = NewUser {
            email: Some("test_insert_with_change@example.com"),
            ..new_user("potato")
        };

        let user = user.create_or_update(&conn).unwrap();
        sign_in_as(&mut req, &user);
        user
    };

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = crate::json::<UserShowPrivateResponse>(&mut response);
    assert_eq!(r.user.email.unwrap(), "test_insert_with_change@example.com");
    assert_eq!(r.user.login, "potato");
    assert!(!r.user.email_verified);
    assert!(r.user.email_verification_sent);

    let body =
        r#"{"user":{"email":"apricot@apricots.apricot","name":"potato","login":"potato","avatar":"https://avatars0.githubusercontent.com","url":"https://github.com/potato","kind":null}}"#;
    let mut response = ok_resp!(middle.call(
        req.with_path(&format!("/api/v1/users/{}", user.id))
            .with_method(Method::Put)
            .with_body(body.as_bytes()),
    ));
    assert!(crate::json::<OkBool>(&mut response).ok);

    logout(&mut req);

    // What if user changes their github user email
    {
        let conn = app.diesel_database.get().unwrap();
        let user = NewUser {
            gh_id: user.gh_id,
            email: Some("banana2@example.com"),
            ..new_user("potato")
        };

        let user = user.create_or_update(&conn).unwrap();
        sign_in_as(&mut req, &user);
    }

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = crate::json::<UserShowPrivateResponse>(&mut response);
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

    let (app, middle) = app();
    let mut req = req(Method::Get, "/me");
    let user = {
        let conn = app.diesel_database.get().unwrap();
        let user = NewUser {
            email: Some("potato2@example.com"),
            ..new_user("potato")
        };

        let user = user.create_or_update(&conn).unwrap();
        sign_in_as(&mut req, &user);
        user
    };

    let email_token = {
        let conn = app.diesel_database.get().unwrap();
        Email::belonging_to(&user)
            .select(emails::token)
            .first::<String>(&*conn)
            .unwrap()
    };

    let mut response = ok_resp!(middle.call(
        req.with_path(&format!("/api/v1/confirm/{}", email_token))
            .with_method(Method::Put),
    ));
    assert!(crate::json::<OkBool>(&mut response).ok);

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = crate::json::<UserShowPrivateResponse>(&mut response);
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

    let (app, middle) = app();
    let mut req = req(Method::Get, "/me");
    {
        let conn = app.diesel_database.get().unwrap();
        let new_user = NewUser {
            email: Some("potahto@example.com"),
            ..new_user("potahto")
        };
        let user = new_user.create_or_update(&conn).unwrap();
        update(Email::belonging_to(&user))
            // Users created before we added verification will have
            // `NULL` in the `token_generated_at` column.
            .set(emails::token_generated_at.eq(None::<NaiveDateTime>))
            .execute(&*conn)
            .unwrap();
        sign_in_as(&mut req, &user);
    }

    let mut response = ok_resp!(middle.call(req.with_path("/api/v1/me").with_method(Method::Get),));
    let r = crate::json::<UserShowPrivateResponse>(&mut response);
    assert_eq!(r.user.email.unwrap(), "potahto@example.com");
    assert!(!r.user.email_verified);
    assert!(!r.user.email_verification_sent);
}
