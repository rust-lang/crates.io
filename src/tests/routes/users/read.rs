use crate::util::{RequestHelper, TestApp};
use crates_io::models::{NewUser, NewOauthGithub};
use crates_io::views::EncodablePublicUser;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct UserShowPublicResponse {
    pub user: EncodablePublicUser,
}

#[tokio::test(flavor = "multi_thread")]
async fn show() {
    let (app, anon, _) = TestApp::init().with_user().await;
    app.db_new_user("Bar").await;

    let json: UserShowPublicResponse = anon.get("/api/v1/users/foo").await.good();
    assert_eq!(json.user.login, "foo");

    let json: UserShowPublicResponse = anon.get("/api/v1/users/bAr").await.good();
    assert_eq!(json.user.login, "Bar");
    assert_eq!(json.user.url, "https://github.com/Bar");
}

#[tokio::test(flavor = "multi_thread")]
async fn show_latest_user_case_insensitively() {
    let (app, anon) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;

    // Please do not delete or modify the setup of this test in order to get it to pass.
    // This setup mimics how GitHub works. If someone abandons a GitHub account, the username is
    // available for anyone to take. We need to support having multiple user accounts with the same
    // `oauth_github.login` in crates.io. `oauth_github.account_id` is stable across renames, so
    // that field should be used for uniquely identifying GitHub accounts whenever possible. For the
    // crates.io/user/{username} pages, the best we can do is show the last crates.io account
    // created with that username.

    let new_user1 = NewUser::builder()
        .gh_id(1)
        .gh_login("foobar")
        .name("I was first then deleted my github account")
        .gh_encrypted_token(&[])
        .build();
    let user1 = new_user1.insert(&mut conn).await.unwrap();
    let linked_account1 = NewOauthGithub::builder()
        .user_id(user1.id)
        .account_id(1)
        .login("foobar")
        .encrypted_token(&[])
        .build();
    linked_account1.insert_or_update(&mut conn).await.unwrap();

    let new_user2 = NewUser::builder()
        .gh_id(2)
        .gh_login("FOOBAR")
        .name("I was second, I took the foobar username on github")
        .gh_encrypted_token(&[])
        .build();
    let user2 = new_user2.insert(&mut conn).await.unwrap();
    let linked_account2 = NewOauthGithub::builder()
        .user_id(user2.id)
        .account_id(2)
        .login("FOOBAR")
        .encrypted_token(&[])
        .build();
    linked_account2.insert_or_update(&mut conn).await.unwrap();

    let json: UserShowPublicResponse = anon.get("/api/v1/users/fOObAr").await.good();
    assert_eq!(
        "I was second, I took the foobar username on github",
        json.user.name.unwrap()
    );
}
