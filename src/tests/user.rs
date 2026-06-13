use crate::TestApp;
use crate::util::github::next_gh_id;
use crate::util::{MockCookieUser, RequestHelper};
use chrono::{DateTime, Utc};
use claims::{assert_err, assert_ok, assert_ok_eq};
use crates_io::controllers::session;
use crates_io::models::{ApiToken, Email, OauthGithub, User};
use crates_io::schema::oauth_github;
use crates_io::util::gh_token_encryption::GitHubTokenEncryption;
use crates_io::util::token::HashedToken;
use crates_io_github::GitHubUser;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use insta::assert_snapshot;
use serde_json::json;

impl crate::util::MockCookieUser {
    async fn confirm_email(&self, email_token: &str) {
        let url = format!("/api/v1/confirm/{email_token}");
        let response = self.put::<()>(&url, &[] as &[u8]).await;
        assert_snapshot!(response.status(), @"200 OK");
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

    let encryption = GitHubTokenEncryption::for_testing();

    // Reuse gh_id but use new gh_login and gh_access_token
    let gh_user = GitHubUser {
        id: gh_id,
        login: "bar".to_string(),
        name: None,
        email: None,
        avatar_url: None,
    };
    let encrypted_token = encryption.encrypt("bar_token")?;
    assert_ok!(session::save_user_to_database(&gh_user, &encrypted_token, emails, &mut conn).await);

    // Use the original API token to find the now updated user
    let hashed_token = assert_ok!(HashedToken::parse(token));
    let api_token = assert_ok!(ApiToken::find_by_api_token(&mut conn, &hashed_token).await);
    let user = assert_ok!(User::find(&conn, api_token.user_id).await);

    assert_eq!(user.gh_login, "bar");
    let decrypted_token = encryption.decrypt(&user.gh_encrypted_token)?;
    assert_eq!(decrypted_token.secret(), "bar_token");

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

    let user_id = session::save_user_to_database(&gh_user, &[], emails, &mut conn).await?;
    let u = User::find(&conn, user_id).await?;

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

    let user_id = session::save_user_to_database(&gh_user, &[], emails, &mut conn).await?;
    let u = User::find(&conn, user_id).await?;

    let again_user_without_github_email = MockCookieUser::new(&app, u);

    let json = again_user_without_github_email.show_me().await;
    assert_eq!(json.user.email.unwrap(), "apricot@apricots.apricot");

    Ok(())
}

/// Given a new user, test that if they sign in with one email, change their email on GitHub, then
/// sign in again, that the email in crates.io will remain set to the original email used on GitHub.
#[tokio::test(flavor = "multi_thread")]
async fn github_with_email_does_not_overwrite_email() -> anyhow::Result<()> {
    use crates_io::schema::emails;

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

    let user_id = session::save_user_to_database(&gh_user, &[], &emails, &mut conn).await?;
    let u = User::find(&conn, user_id).await?;

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
    use crates_io::schema::emails;

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

    let user_id = session::save_user_to_database(&gh_user, &[], emails, &mut conn).await?;
    let u = User::find(&conn, user_id).await?;

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
    use crates_io::schema::emails;
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

    let user_id = session::save_user_to_database(&gh_user, &[], emails, &mut conn).await?;
    let u = User::find(&conn, user_id).await?;

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

// To assist in the transition from `users.gh_login` to `users.username`, also write the
// `GitHubUser`'s `login` to `users.username`.
#[tokio::test(flavor = "multi_thread")]
async fn also_write_to_users_username() -> anyhow::Result<()> {
    let (app, _) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;
    let encryption = GitHubTokenEncryption::for_testing();
    let gh_id = next_gh_id();
    let email = "potahto@example.com";
    let emails = &app.as_inner().emails;

    // Simulate logging in via GitHub. Don't use app.db_new_user because it inserts a user record
    // directly into the database and we want to test the OAuth flow here.
    let gh_user = GitHubUser {
        id: gh_id,
        login: "arbitrary_username".to_string(),
        name: None,
        email: Some(email.to_string()),
        avatar_url: None,
    };
    let encrypted_token = encryption.encrypt("some random token")?;
    let uid = session::save_user_to_database(&gh_user, &encrypted_token, emails, &mut conn).await?;
    let u = User::find(&conn, uid).await?;

    assert_eq!(u.username, "arbitrary_username");

    Ok(())
}

// To assist in eventually someday allowing OAuth with more than GitHub, verify that we're writing
// the GitHub info to both the `users` and `oauth_github` tables.
#[tokio::test(flavor = "multi_thread")]
async fn write_to_users_and_oauth_github() -> anyhow::Result<()> {
    let (app, _) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;
    let encryption = GitHubTokenEncryption::for_testing();
    let gh_id = next_gh_id();
    let gh_login = "arbitrary_username".to_string();
    let gh_display_name = "Arbitrary Username".to_string();
    let gh_avatar = "http://example.com/icon-the-first.png".to_string();
    let gh_token = "some random token";
    let email = "potahto@example.com";
    let emails = &app.as_inner().emails;

    // Simulate logging in via GitHub. Don't use app.db_new_user because it inserts a user record
    // directly into the database and we want to test the OAuth flow here.
    let gh_user = GitHubUser {
        id: gh_id,
        login: gh_login.clone(),
        name: Some(gh_display_name.clone()),
        email: Some(email.to_string()),
        avatar_url: Some(gh_avatar.clone()),
    };
    let encrypted_token = encryption.encrypt(gh_token)?;
    let uid = session::save_user_to_database(&gh_user, &encrypted_token, emails, &mut conn).await?;
    let u = User::find(&conn, uid).await?;
    assert_eq!(u.username, gh_login);
    assert_eq!(u.name.unwrap(), gh_display_name);
    assert_eq!(u.gh_id, gh_id);
    assert_eq!(u.gh_login, gh_login);
    assert_eq!(u.gh_avatar.unwrap(), gh_avatar);
    let decrypted_token = encryption.decrypt(&u.gh_encrypted_token)?;
    assert_eq!(decrypted_token.secret(), gh_token);

    let oauth_github_records: Vec<OauthGithub> = oauth_github::table.load(&mut conn).await.unwrap();
    assert_eq!(oauth_github_records.len(), 1);
    let oauth_github = &oauth_github_records[0];
    assert_eq!(oauth_github.user_id, u.id);
    assert_eq!(oauth_github.account_id, gh_id as i64);
    assert_eq!(oauth_github.login, gh_login);
    assert_eq!(oauth_github.avatar.as_ref().unwrap(), &gh_avatar);
    let decrypted_token = encryption.decrypt(&oauth_github.encrypted_token)?;
    assert_eq!(decrypted_token.secret(), gh_token);

    // Log in again with the same gh_id but different login, avatar, and token; these should get
    // updated in both the `users` and `oauth_github` tables.
    let different_gh_login = "i_changed_my_username".to_string();
    let different_gh_display_name = "Someone Else".to_string();
    let different_gh_avatar = "http://example.com/icon-the-second.png".to_string();
    let different_gh_token = "a different token";
    let gh_user = GitHubUser {
        id: gh_id,
        login: different_gh_login.clone(),
        name: Some(different_gh_display_name.clone()),
        email: Some(email.to_string()),
        avatar_url: Some(different_gh_avatar.clone()),
    };
    let encrypted_token = encryption.encrypt(different_gh_token)?;
    let uid = session::save_user_to_database(&gh_user, &encrypted_token, emails, &mut conn).await?;
    let u = User::find(&conn, uid).await?;
    assert_eq!(u.username, different_gh_login);
    assert_eq!(u.name.unwrap(), different_gh_display_name);
    assert_eq!(u.gh_id, gh_id);
    assert_eq!(u.gh_login, different_gh_login);
    assert_eq!(u.gh_avatar.unwrap(), different_gh_avatar);
    let decrypted_token = encryption.decrypt(&u.gh_encrypted_token)?;
    assert_eq!(decrypted_token.secret(), different_gh_token);

    let oauth_github_records: Vec<OauthGithub> = oauth_github::table.load(&mut conn).await.unwrap();
    // There still should only be one `oauth_github` record that got updated, not a new insertion
    assert_eq!(oauth_github_records.len(), 1);
    let oauth_github = &oauth_github_records[0];
    assert_eq!(oauth_github.user_id, u.id);
    assert_eq!(oauth_github.login, different_gh_login);
    assert_eq!(oauth_github.avatar.as_ref().unwrap(), &different_gh_avatar,);
    let decrypted_token = encryption.decrypt(&oauth_github.encrypted_token)?;
    assert_eq!(decrypted_token.secret(), different_gh_token);

    // Now that the user has renamed their account on GitHub, someone else can claim it and log in
    // to crates.io with it (with a different GitHub ID)
    let new_gh_id = gh_id + 1;
    let gh_user = GitHubUser {
        id: new_gh_id,
        login: gh_login.clone(),
        name: None,
        email: Some(email.to_string()),
        avatar_url: None,
    };
    let another_gh_token = "a different random token";
    let encrypted_token = encryption.encrypt(another_gh_token)?;
    let uid = session::save_user_to_database(&gh_user, &encrypted_token, emails, &mut conn).await?;
    let u = User::find(&conn, uid).await?;

    assert_eq!(u.gh_login, gh_login);
    assert_eq!(u.gh_id, new_gh_id);

    let oauth_github_records: Vec<OauthGithub> = oauth_github::table.load(&mut conn).await.unwrap();
    assert_eq!(oauth_github_records.len(), 2);
    let additional_user_oauth_github = oauth_github_records
        .iter()
        .find(|gh| *gh.id() == new_gh_id as i64)
        .unwrap();

    assert_eq!(additional_user_oauth_github.user_id, u.id);
    assert_eq!(additional_user_oauth_github.account_id, new_gh_id as i64);
    assert_eq!(additional_user_oauth_github.login, gh_login);
    assert!(additional_user_oauth_github.avatar.is_none());
    let decrypted_token = encryption.decrypt(&additional_user_oauth_github.encrypted_token)?;
    assert_eq!(decrypted_token.secret(), another_gh_token);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn existing_user_can_log_in_during_read_only_mode() -> anyhow::Result<()> {
    let (app, _) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;
    let emails = &app.as_inner().emails;

    let gh_user = GitHubUser {
        id: next_gh_id(),
        login: "github_user".to_string(),
        name: Some("My Name".to_string()),
        email: None,
        avatar_url: None,
    };

    // Create the user and its `oauth_github` record while the database is writable.
    let user_id = session::save_user_to_database(&gh_user, b"token", emails, &mut conn).await?;

    // Switch the connection into read-only mode, mirroring how the app configures
    // read-only connections in `ConnectionConfig::apply()`.
    diesel::sql_query("SET default_transaction_read_only = 't'")
        .execute(&mut conn)
        .await?;

    // Logging in again as an existing user must still succeed by falling back to a lookup, even
    // though the write attempts fail in read-only mode.
    let result = session::save_user_to_database(&gh_user, b"token", emails, &mut conn).await;

    assert_ok_eq!(result, user_id);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn new_user_cannot_log_in_during_read_only_mode() -> anyhow::Result<()> {
    let (app, _) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;
    let emails = &app.as_inner().emails;

    let gh_user = GitHubUser {
        id: next_gh_id(),
        login: "github_user".to_string(),
        name: Some("My Name".to_string()),
        email: None,
        avatar_url: None,
    };

    // Switch the connection into read-only mode, mirroring how the app configures
    // read-only connections in `ConnectionConfig::apply()`.
    diesel::sql_query("SET default_transaction_read_only = 't'")
        .execute(&mut conn)
        .await?;

    // Logging in as a new user can't work in read-only mode.
    let result = session::save_user_to_database(&gh_user, b"token", emails, &mut conn).await;

    let error = assert_err!(result);
    assert_snapshot!(error, @"cannot execute UPDATE in a read-only transaction");

    Ok(())
}
