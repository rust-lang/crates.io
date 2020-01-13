use crate::{
    builders::{CrateBuilder, VersionBuilder},
    new_user,
    util::{MockCookieUser, RequestHelper, Response},
    OkBool, TestApp,
};
use cargo_registry::{
    models::{Email, NewUser, User},
    schema::crate_owners,
    views::{EncodablePrivateUser, EncodablePublicUser, EncodableVersion, OwnedCrate},
};

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
    pub owned_crates: Vec<OwnedCrate>,
}

#[derive(Deserialize)]
struct UserStats {
    total_downloads: i64,
}

#[derive(Serialize)]
struct EmailNotificationsUpdate {
    id: i32,
    email_notifications: bool,
}

impl crate::util::MockCookieUser {
    fn show_me(&self) -> UserShowPrivateResponse {
        let url = "/api/v1/me";
        self.get(url).good()
    }

    fn update_email(&self, email: &str) -> OkBool {
        let model = self.as_model();
        self.update_email_more_control(model.id, Some(email)).good()
    }

    // TODO: I don't like the name of this method or the one above; this is starting to look like
    // a builder might help? I want to explore alternative abstractions in any case
    fn update_email_more_control(&self, user_id: i32, email: Option<&str>) -> Response<OkBool> {
        // When updating your email in crates.io, the request goes to the user route with PUT.
        // Ember sends all the user attributes. We check to make sure the ID in the URL matches
        // the ID of the currently logged in user, then we ignore everything but the email address.
        let body = json!({"user": {
            "email": email,
            "name": "Arbitrary Name",
            "login": "arbitrary_login",
            "avatar": "https://arbitrary.com/img.jpg",
            "url": "https://arbitrary.com",
            "kind": null
        }});
        let url = format!("/api/v1/users/{}", user_id);
        self.put(&url, body.to_string().as_bytes())
    }

    fn confirm_email(&self, email_token: &str) -> OkBool {
        let url = format!("/api/v1/confirm/{}", email_token);
        self.put(&url, &[]).good()
    }

    fn update_email_notifications(&self, updates: Vec<EmailNotificationsUpdate>) -> OkBool {
        self.put(
            "/api/v1/me/email_notifications",
            json!(updates).to_string().as_bytes(),
        )
        .good()
    }
}

impl crate::util::MockAnonymousUser {
    // TODO: Refactor to get rid of this duplication with the same method implemented on
    // MockCookieUser
    fn update_email_more_control(&self, user_id: i32, email: Option<&str>) -> Response<OkBool> {
        // When updating your email in crates.io, the request goes to the user route with PUT.
        // Ember sends all the user attributes. We check to make sure the ID in the URL matches
        // the ID of the currently logged in user, then we ignore everything but the email address.
        let body = json!({"user": {
            "email": email,
            "name": "Arbitrary Name",
            "login": "arbitrary_login",
            "avatar": "https://arbitrary.com/img.jpg",
            "url": "https://arbitrary.com",
            "kind": null
        }});
        let url = format!("/api/v1/users/{}", user_id);
        self.put(&url, body.to_string().as_bytes())
    }
}

#[test]
fn auth_gives_a_token() {
    let (_, anon) = TestApp::init().empty();
    let json: AuthResponse = anon.get("/api/private/session/begin").good();
    assert!(json.url.contains(&json.state));
}

#[test]
fn access_token_needs_data() {
    let (_, anon) = TestApp::init().empty();
    let json = anon
        .get::<()>("/api/private/session/authorize")
        .bad_with_status(400);
    assert!(json.errors[0].detail.contains("invalid state"));
}

#[test]
fn me() {
    let url = "/api/v1/me";
    let (app, anon) = TestApp::init().empty();
    anon.get(url).assert_forbidden();

    let user = app.db_new_user("foo");
    let json = user.show_me();

    assert_eq!(json.owned_crates.len(), 0);

    app.db(|conn| {
        CrateBuilder::new("foo_my_packages", user.as_model().id).expect_build(conn);
        assert_eq!(json.user.email, user.as_model().email(conn).unwrap());
    });
    let updated_json = user.show_me();

    assert_eq!(updated_json.owned_crates.len(), 1);
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
            Some("I was first then deleted my github account"),
            None,
            "bar"
        )
        .create_or_update(None, conn));
        t!(NewUser::new(
            2,
            "FOOBAR",
            Some("I was second, I took the foobar username on github"),
            None,
            "bar"
        )
        .create_or_update(None, conn));
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
        .bad_with_status(400);
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

        let mut no_longer_my_krate = CrateBuilder::new("nacho", user.id).expect_build(conn);
        no_longer_my_krate.downloads = 5;
        update(&no_longer_my_krate)
            .set(&no_longer_my_krate)
            .execute(conn)
            .unwrap();
        no_longer_my_krate
            .owner_remove(app.as_inner(), conn, user, &user.gh_login)
            .unwrap();
    });

    let url = format!("/api/v1/users/{}/stats", user.id);
    let stats: UserStats = anon.get(&url).good();
    // does not include crates user never owned (2) or no longer owns (5)
    assert_eq!(stats.total_downloads, 30);
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
        t!(NewUser::new(gh_id, "bar", None, None, "bar_token").create_or_update(None, conn));

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
fn github_without_email_does_not_overwrite_email() {
    let (app, _) = TestApp::init().empty();

    // Simulate logging in via GitHub with an account that has no email.
    // Because faking GitHub is terrible, call what GithubUser::save_to_database does directly.
    // Don't use app.db_new_user because it adds a verified email.
    let user_without_github_email = app.db(|conn| {
        let u = new_user("arbitrary_username");
        let u = u.create_or_update(None, conn).unwrap();
        MockCookieUser::new(&app, u)
    });
    let user_without_github_email_model = user_without_github_email.as_model();

    let json = user_without_github_email.show_me();
    // Check that the setup is correct and the user indeed has no email
    assert_eq!(json.user.email, None);

    // Add an email address in crates.io
    user_without_github_email.update_email("apricot@apricots.apricot");

    // Simulate the same user logging in via GitHub again, still with no email in GitHub.
    let again_user_without_github_email = app.db(|conn| {
        let u = NewUser {
            // Use the same github ID to link to the existing account
            gh_id: user_without_github_email_model.gh_id,
            // new_user uses a None email; the rest of the fields are arbitrary
            ..new_user("arbitrary_username")
        };
        let u = u.create_or_update(None, conn).unwrap();
        MockCookieUser::new(&app, u)
    });

    let json = again_user_without_github_email.show_me();
    assert_eq!(json.user.email.unwrap(), "apricot@apricots.apricot");
}

/* Given a new user, test that if they sign in with one email, change their email on GitHub, then
   sign in again, that the email in crates.io will remain set to the original email used on GitHub.
*/
#[test]
fn github_with_email_does_not_overwrite_email() {
    use cargo_registry::schema::emails;

    let (app, _, user) = TestApp::init().with_user();
    let model = user.as_model();
    let original_email = app.db(|conn| {
        Email::belonging_to(model)
            .select(emails::email)
            .first::<String>(&*conn)
            .unwrap()
    });

    let new_github_email = "new-email-in-github@example.com";

    // Simulate logging in to crates.io after changing your email in GitHub
    let user_with_different_email_in_github = app.db(|conn| {
        let u = NewUser {
            // Use the same github ID to link to the existing account
            gh_id: model.gh_id,
            // the rest of the fields are arbitrary
            ..new_user("arbitrary_username")
        };
        let u = u.create_or_update(Some(new_github_email), conn).unwrap();
        MockCookieUser::new(&app, u)
    });

    let json = user_with_different_email_in_github.show_me();
    assert_eq!(json.user.email, Some(original_email));
}

/*  Given a crates.io user, check that the user's email can be
    updated in the database (PUT /user/:user_id), then check
    that the updated email is sent back to the user (GET /me).
*/
#[test]
fn test_email_get_and_put() {
    let (_app, _anon, user) = TestApp::init().with_user();

    let json = user.show_me();
    assert_eq!(json.user.email.unwrap(), "something@example.com");

    user.update_email("mango@mangos.mango");

    let json = user.show_me();
    assert_eq!(json.user.email.unwrap(), "mango@mangos.mango");
    assert!(!json.user.email_verified);
    assert!(json.user.email_verification_sent);
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
    let (_app, _anon, user) = TestApp::init().with_user();
    let model = user.as_model();

    let json = user
        .update_email_more_control(model.id, Some(""))
        .bad_with_status(400);
    assert!(
        json.errors[0].detail.contains("empty email rejected"),
        "{:?}",
        json.errors
    );

    let json = user
        .update_email_more_control(model.id, None)
        .bad_with_status(400);

    assert!(
        json.errors[0].detail.contains("empty email rejected"),
        "{:?}",
        json.errors
    );
}

/*  Check to make sure that neither other signed in users nor anonymous users can edit another
    user's email address.

    If an attempt is made, update_user.rs will return an error indicating that the current user
    does not match the requested user.
*/
#[test]
fn test_other_users_cannot_change_my_email() {
    let (app, anon, user) = TestApp::init().with_user();
    let another_user = app.db_new_user("not_me");
    let another_user_model = another_user.as_model();

    let json = user
        .update_email_more_control(
            another_user_model.id,
            Some("pineapple@pineapples.pineapple"),
        )
        .bad_with_status(400);
    assert!(
        json.errors[0]
            .detail
            .contains("current user does not match requested user",),
        "{:?}",
        json.errors
    );

    anon.update_email_more_control(
        another_user_model.id,
        Some("pineapple@pineapples.pineapple"),
    )
    .bad_with_status(403);
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

    let (app, _) = TestApp::init().empty();

    // Simulate logging in via GitHub. Don't use app.db_new_user because it inserts a verified
    // email directly into the database and we want to test the verification flow here.
    let email = "potato2@example.com";

    let user = app.db(|conn| {
        let u = NewUser {
            ..new_user("arbitrary_username")
        };
        let u = u.create_or_update(Some(email), conn).unwrap();
        MockCookieUser::new(&app, u)
    });
    let user_model = user.as_model();

    let email_token = app.db(|conn| {
        Email::belonging_to(user_model)
            .select(emails::token)
            .first::<String>(&*conn)
            .unwrap()
    });

    user.confirm_email(&email_token);

    let json = user.show_me();
    assert_eq!(json.user.email.unwrap(), "potato2@example.com");
    assert!(json.user.email_verified);
    assert!(json.user.email_verification_sent);
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

    let (app, _) = TestApp::init().empty();

    // Simulate logging in via GitHub. Don't use app.db_new_user because it inserts a verified
    // email directly into the database and we want to test the verification flow here.
    let email = "potahto@example.com";
    let user = app.db(|conn| {
        let u = NewUser {
            ..new_user("arbitrary_username")
        };
        let u = u.create_or_update(Some(email), conn).unwrap();
        update(Email::belonging_to(&u))
            // Users created before we added verification will have
            // `NULL` in the `token_generated_at` column.
            .set(emails::token_generated_at.eq(None::<NaiveDateTime>))
            .execute(conn)
            .unwrap();
        MockCookieUser::new(&app, u)
    });

    let json = user.show_me();
    assert_eq!(json.user.email.unwrap(), "potahto@example.com");
    assert!(!json.user.email_verified);
    assert!(!json.user.email_verification_sent);
}

#[test]
fn test_user_owned_crates_doesnt_include_deleted_ownership() {
    let (app, _, user) = TestApp::init().with_user();
    let user_model = user.as_model();

    app.db(|conn| {
        let krate = CrateBuilder::new("foo_my_packages", user_model.id).expect_build(conn);
        krate
            .owner_remove(app.as_inner(), conn, user_model, &user_model.gh_login)
            .unwrap();
    });

    let json = user.show_me();
    assert_eq!(json.owned_crates.len(), 0);
}

/* A user should be able to update the email notifications for crates they own. Only the crates that
   were sent in the request should be updated to the corresponding `email_notifications` value.
*/
#[test]
fn test_update_email_notifications() {
    let (app, _, user) = TestApp::init().with_user();

    let my_crates = app.db(|conn| {
        vec![
            CrateBuilder::new("test_package", user.as_model().id).expect_build(&conn),
            CrateBuilder::new("another_package", user.as_model().id).expect_build(&conn),
        ]
    });

    let a_id = my_crates.get(0).unwrap().id;
    let b_id = my_crates.get(1).unwrap().id;

    // Update crate_a: email_notifications = false
    // crate_a should be false, crate_b should be true
    user.update_email_notifications(vec![EmailNotificationsUpdate {
        id: a_id,
        email_notifications: false,
    }]);
    let json = user.show_me();

    assert_eq!(
        json.owned_crates
            .iter()
            .find(|c| c.id == a_id)
            .unwrap()
            .email_notifications,
        false
    );
    assert_eq!(
        json.owned_crates
            .iter()
            .find(|c| c.id == b_id)
            .unwrap()
            .email_notifications,
        true
    );

    // Update crate_b: email_notifications = false
    // Both should be false now
    user.update_email_notifications(vec![EmailNotificationsUpdate {
        id: b_id,
        email_notifications: false,
    }]);
    let json = user.show_me();

    assert_eq!(
        json.owned_crates
            .iter()
            .find(|c| c.id == a_id)
            .unwrap()
            .email_notifications,
        false
    );
    assert_eq!(
        json.owned_crates
            .iter()
            .find(|c| c.id == b_id)
            .unwrap()
            .email_notifications,
        false
    );

    // Update crate_a and crate_b: email_notifications = true
    // Both should be true
    user.update_email_notifications(vec![
        EmailNotificationsUpdate {
            id: a_id,
            email_notifications: true,
        },
        EmailNotificationsUpdate {
            id: b_id,
            email_notifications: true,
        },
    ]);
    let json = user.show_me();

    json.owned_crates.iter().for_each(|c| {
        assert!(c.email_notifications);
    })
}

/* A user should not be able to update the `email_notifications` value for a crate that is not
   owned by them.
*/
#[test]
fn test_update_email_notifications_not_owned() {
    let (app, _, user) = TestApp::init().with_user();

    let not_my_crate = app.db(|conn| {
        let u = new_user("arbitrary_username")
            .create_or_update(None, &conn)
            .unwrap();
        CrateBuilder::new("test_package", u.id).expect_build(&conn)
    });

    user.update_email_notifications(vec![EmailNotificationsUpdate {
        id: not_my_crate.id,
        email_notifications: false,
    }]);

    let email_notifications = app
        .db(|conn| {
            crate_owners::table
                .select(crate_owners::email_notifications)
                .filter(crate_owners::crate_id.eq(not_my_crate.id))
                .first::<bool>(&*conn)
        })
        .unwrap();

    // There should be no change to the `email_notifications` value for a crate not belonging to me
    assert!(email_notifications);
}
