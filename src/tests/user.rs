use crate::controllers::session;
use crate::models::{ApiToken, Email, User};
use crate::tests::TestApp;
use crate::tests::util::github::next_gh_id;
use crate::tests::util::{MockCookieUser, RequestHelper};
use crate::util::token::HashedToken;
use chrono::{DateTime, Utc};
use crates_io_github::GitHubUser;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;
use secrecy::ExposeSecret;
use serde_json::json;

impl crate::tests::util::MockCookieUser {
    async fn confirm_email(&self, email_token: &str) {
        let url = format!("/api/v1/confirm/{email_token}");
        let response = self.put::<()>(&url, &[] as &[u8]).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn updating_existing_user_doesnt_change_api_token() -> anyhow::Result<()> {
    let (app, _, user, token) = TestApp::init().with_token().await;
    let emails = &app.as_inner().emails;
    let mut conn = app.db_conn().await;
    let gh_id = user.as_model().gh_id;
    let token = token.plaintext();

    // Reuse gh_id but use new gh_login and gh_access_token
    let gh_user = GitHubUser {
        id: gh_id,
        login: "bar".to_string(),
        name: None,
        email: None,
        avatar_url: None,
    };
    assert_ok!(session::save_user_to_database(&gh_user, "bar_token", emails, &mut conn).await);

    // Use the original API token to find the now updated user
    let hashed_token = assert_ok!(HashedToken::parse(token));
    let api_token = assert_ok!(ApiToken::find_by_api_token(&mut conn, &hashed_token).await);
    let user = assert_ok!(User::find(&mut conn, api_token.user_id).await);

    assert_eq!(user.gh_login, "bar");
    assert_eq!(user.gh_access_token.expose_secret(), "bar_token");

    Ok(())
}

/// Given a GitHub user, check that if the user logs in,
/// updates their email, logs out, then logs back in, the
/// email they added to crates.io will not be overwritten
/// by the information sent by GitHub.
///
/// This bug is problematic if the user's email preferences
/// are set to private on GitHub, as GitHub will always
/// send none as the email and we will end up inadvertently
/// deleting their email when they sign back in.
#[tokio::test(flavor = "multi_thread")]
async fn github_without_email_does_not_overwrite_email() -> anyhow::Result<()> {
    let (app, _) = TestApp::init().empty().await;
    let emails = &app.as_inner().emails;
    let mut conn = app.db_conn().await;

    // Simulate logging in via GitHub with an account that has no email.

    // Because faking GitHub is terrible, call what GitHubUser::save_to_database does directly.
    // Don't use app.db_new_user because it adds a verified email.
    let gh_id = next_gh_id();
    let gh_user = GitHubUser {
        id: gh_id,
        login: "arbitrary_username".to_string(),
        name: None,
        email: None,
        avatar_url: None,
    };

    let u =
        session::save_user_to_database(&gh_user, "some random token", emails, &mut conn).await?;

    let user_without_github_email = MockCookieUser::new(&app, u);

    let json = user_without_github_email.show_me().await;
    // Check that the setup is correct and the user indeed has no email
    assert_eq!(json.user.email, None);

    // Add an email address in crates.io
    user_without_github_email
        .update_email("apricot@apricots.apricot")
        .await;

    // Simulate the same user logging in via GitHub again, still with no email in GitHub.

    let gh_user = GitHubUser {
        id: gh_id,
        login: "arbitrary_username".to_string(),
        name: None,
        email: None,
        avatar_url: None,
    };

    let u =
        session::save_user_to_database(&gh_user, "some random token", emails, &mut conn).await?;

    let again_user_without_github_email = MockCookieUser::new(&app, u);

    let json = again_user_without_github_email.show_me().await;
    assert_eq!(json.user.email.unwrap(), "apricot@apricots.apricot");

    Ok(())
}

/// Given a new user, test that if they sign in with one email, change their email on GitHub, then
/// sign in again, that the email in crates.io will remain set to the original email used on GitHub.
#[tokio::test(flavor = "multi_thread")]
async fn github_with_email_does_not_overwrite_email() -> anyhow::Result<()> {
    use crate::schema::emails;

    let (app, _, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;

    let model = user.as_model();

    let original_email: String = Email::belonging_to(model)
        .select(emails::email)
        .first(&mut conn)
        .await?;

    let new_github_email = "new-email-in-github@example.com";

    // Simulate logging in to crates.io after changing your email in GitHub

    let emails = app.as_inner().emails.clone();

    let gh_user = GitHubUser {
        // Use the same github ID to link to the existing account
        id: model.gh_id,
        login: "arbitrary_username".to_string(),
        name: None,
        email: Some(new_github_email.to_string()),
        avatar_url: None,
    };

    let u =
        session::save_user_to_database(&gh_user, "some random token", &emails, &mut conn).await?;

    let user_with_different_email_in_github = MockCookieUser::new(&app, u);

    let json = user_with_different_email_in_github.show_me().await;
    assert_eq!(json.user.email, Some(original_email));

    Ok(())
}

/// Given a crates.io user, check that the user's email can be
/// updated in the database (PUT /user/{user_id}), then check
/// that the updated email is sent back to the user (GET /me).
#[tokio::test(flavor = "multi_thread")]
async fn test_email_get_and_put() -> anyhow::Result<()> {
    let (_app, _anon, user) = TestApp::init().with_user().await;

    let json = user.show_me().await;
    assert_eq!(json.user.email.unwrap(), "foo@example.com");

    user.update_email("mango@mangos.mango").await;

    let json = user.show_me().await;
    assert_eq!(json.user.email.unwrap(), "mango@mangos.mango");
    assert!(!json.user.email_verified);
    assert!(json.user.email_verification_sent);

    Ok(())
}

/// Given a new user, test that their email can be added
/// to the email table and a token for the email is generated
/// and added to the token table. When /confirm/{email_token} is
/// requested, check that the response back is ok, and that
/// the email_verified field on user is now set to true.
#[tokio::test(flavor = "multi_thread")]
async fn test_confirm_user_email() -> anyhow::Result<()> {
    use crate::schema::emails;

    let (app, _) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;

    // Simulate logging in via GitHub. Don't use app.db_new_user because it inserts a verified
    // email directly into the database and we want to test the verification flow here.
    let email = "potato2@example.com";

    let emails = &app.as_inner().emails;

    let gh_user = GitHubUser {
        id: next_gh_id(),
        login: "arbitrary_username".to_string(),
        name: None,
        email: Some(email.to_string()),
        avatar_url: None,
    };

    let u =
        session::save_user_to_database(&gh_user, "some random token", emails, &mut conn).await?;

    let user = MockCookieUser::new(&app, u);
    let user_model = user.as_model();

    let email_token: String = Email::belonging_to(user_model)
        .select(emails::token)
        .first(&mut conn)
        .await?;

    user.confirm_email(&email_token).await;

    let json = user.show_me().await;
    assert_eq!(json.user.email.unwrap(), "potato2@example.com");
    assert!(json.user.email_verified);
    assert!(json.user.email_verification_sent);

    Ok(())
}

/// Given a user who existed before we added email confirmation,
/// test that `email_verification_sent` is false so that we don't
/// make the user think we've sent an email when we haven't.
#[tokio::test(flavor = "multi_thread")]
async fn test_existing_user_email() -> anyhow::Result<()> {
    use crate::schema::emails;
    use diesel::update;

    let (app, _) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;

    // Simulate logging in via GitHub. Don't use app.db_new_user because it inserts a verified
    // email directly into the database and we want to test the verification flow here.
    let email = "potahto@example.com";

    let emails = &app.as_inner().emails;

    let gh_user = GitHubUser {
        id: next_gh_id(),
        login: "arbitrary_username".to_string(),
        name: None,
        email: Some(email.to_string()),
        avatar_url: None,
    };

    let u =
        session::save_user_to_database(&gh_user, "some random token", emails, &mut conn).await?;

    update(Email::belonging_to(&u))
        // Users created before we added verification will have
        // `NULL` in the `token_generated_at` column.
        .set(emails::token_generated_at.eq(None::<DateTime<Utc>>))
        .execute(&mut conn)
        .await?;
    let user = MockCookieUser::new(&app, u);

    let json = user.show_me().await;
    assert_eq!(json.user.email.unwrap(), "potahto@example.com");
    assert!(!json.user.email_verified);
    assert!(!json.user.email_verification_sent);

    Ok(())
}
