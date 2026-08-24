use crate::util::{RequestHelper, TestApp};
use claims::{assert_none, assert_ok, assert_some, assert_some_eq};
use crates_io::models::{NewUser, User};
use crates_io::views::{EncodableLinkedAccount, EncodablePublicUser};
use crates_io_test_utils::builders::{OauthGithubBuilder, UserBuilder};
use insta::{assert_json_snapshot, assert_snapshot};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct UserShowPublicResponse {
    pub user: EncodablePublicUser,
    pub linked_accounts: Option<Vec<EncodableLinkedAccount>>,
}

#[tokio::test(flavor = "multi_thread")]
async fn show() {
    let (app, anon, _) = TestApp::init().with_user().await;

    let builder = UserBuilder::new()
        .with_username("crates-Bar")
        .with_gh_login("github-Bar");

    app.db_new_user_from_builder(builder).await;

    let json: UserShowPublicResponse = anon.get("/api/v1/users/foo").await.good();
    assert_eq!(json.user.login, "foo");
    assert!(json.user.github_username_matches);
    assert_none!(json.linked_accounts);

    let url = "/api/v1/users/foo?include=linked_accounts";
    let response = anon.get::<()>(url).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), {
        ".user.created_at" => "[datetime]",
        ".linked_accounts[].account_id" => insta::dynamic_redaction(|value, _path| {
            let account_id = assert_some!(value.as_str());
            assert_ok!(account_id.parse::<i64>());
            "[account-id]"
        }),
    });

    let response = anon.get::<()>("/api/v1/users/foo?include=unknown").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(
        response.text(),
        @r#"{"errors":[{"detail":"invalid component for ?include= (expected 'linked_accounts')"}]}"#
    );

    // Lookup by username is case insensitive; returned data uses capitalization in database
    let json: UserShowPublicResponse = anon.get("/api/v1/users/crates_bar").await.good();
    assert_eq!(json.user.login, "crates-Bar");
    assert_eq!(json.user.url, "https://github.com/github-Bar");
    assert!(!json.user.github_username_matches);

    // GitHub logins are not used to resolve crates.io users
    let response = anon.get::<()>("/api/v1/users/github-Bar").await;
    assert_snapshot!(response.status(), @"404 Not Found");

    // Username not in database results in 404
    let response = anon.get::<()>("/api/v1/users/not_a_user").await;
    assert_snapshot!(response.status(), @"404 Not Found");
}

#[tokio::test(flavor = "multi_thread")]
async fn show_latest_user_case_insensitively() {
    let (app, anon) = TestApp::init().empty().await;
    let conn = app.db_conn().await;

    // Please do not delete or modify the setup of this test in order to get it to pass.
    // This setup mimics how GitHub works. If someone abandons a GitHub account, the username is
    // available for anyone to take. We need to support having multiple user accounts
    // with the same gh_login in crates.io. `gh_id` is stable across renames, so that field
    // should be used for uniquely identifying GitHub accounts whenever possible. For the
    // crates.io/user/{username} pages, the best we can do is show the last crates.io account
    // created with that username.

    let user1 = UserBuilder::new()
        .with_username("foobar")
        .with_display_name("I was first then deleted my github account")
        .new_user();
    let user1_id = user1.insert(&conn).await.unwrap();
    let user1 = User::find(&conn, user1_id).await.unwrap();
    OauthGithubBuilder::for_user(&user1).insert(&conn).await;

    let user2 = UserBuilder::new()
        .with_username("FOOBAR")
        .with_display_name("I was second, I took the foobar username on github")
        .new_user();
    let user2_id = user2.insert(&conn).await.unwrap();
    let user2 = User::find(&conn, user2_id).await.unwrap();
    OauthGithubBuilder::for_user(&user2)
        .with_login("FOO-BAR")
        .insert(&conn)
        .await;

    let json: UserShowPublicResponse = anon.get("/api/v1/users/fOObAr").await.good();
    assert_eq!(
        "I was second, I took the foobar username on github",
        json.user.name.unwrap()
    );
    assert!(!json.user.github_username_matches);
}

#[tokio::test(flavor = "multi_thread")]
async fn user_without_github_account() {
    let (app, anon) = TestApp::init().empty().await;
    let conn = app.db_conn().await;

    let new_user = NewUser::builder()
        // The gh_id column will eventually be removed; there are currently records in production
        // that have `-1` for their `gh_id` because the associated GitHub accounts have been deleted
        .gh_id(-1)
        .gh_login("foobar")
        .username("foobar")
        .name("I deleted my github account")
        .build();
    new_user.insert(&conn).await.unwrap();
    // This user doesn't have a linked record in `oauth_github`

    // The crates.io username still exists
    let url = "/api/v1/users/fOObAr?include=linked_accounts";
    let json: UserShowPublicResponse = anon.get(url).await.good();
    assert_eq!("I deleted my github account", json.user.name.unwrap());
    assert!(!json.user.github_username_matches);
    assert_some_eq!(json.linked_accounts, Vec::new());
}
