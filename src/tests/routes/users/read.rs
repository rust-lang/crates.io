use crate::util::{RequestHelper, TestApp};
use claims::assert_ok;
use crates_io::models::NewUser;
use crates_io::schema::users;
use crates_io::views::EncodablePublicUser;
use diesel_async::RunQueryDsl;
use insta::assert_snapshot;
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

    // Lookup by username is case insensitive; returned data uses capitalization in database
    let json: UserShowPublicResponse = anon.get("/api/v1/users/bAr").await.good();
    assert_eq!(json.user.login, "Bar");
    assert_eq!(json.user.url, "https://github.com/Bar");

    // Username not in database results in 404
    let response = anon.get::<()>("/api/v1/users/not_a_user").await;
    assert_snapshot!(response.status(), @"404 Not Found");
}

#[tokio::test(flavor = "multi_thread")]
async fn show_latest_user_case_insensitively() {
    let (app, anon) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;

    // Please do not delete or modify the setup of this test in order to get it to pass.
    // This setup mimics how GitHub works. If someone abandons a GitHub account, the username is
    // available for anyone to take. We need to support having multiple user accounts
    // with the same gh_login in crates.io. `gh_id` is stable across renames, so that field
    // should be used for uniquely identifying GitHub accounts whenever possible. For the
    // crates.io/user/{username} pages, the best we can do is show the last crates.io account
    // created with that username.

    let user1 = NewUser::builder()
        .gh_id(1)
        .gh_login("foobar")
        .name("I was first then deleted my github account")
        .gh_encrypted_token(&[])
        .build();

    let user2 = NewUser::builder()
        .gh_id(2)
        .gh_login("FOOBAR")
        .name("I was second, I took the foobar username on github")
        .gh_encrypted_token(&[])
        .build();

    assert_ok!(
        diesel::insert_into(users::table)
            .values(&vec![user1, user2])
            .execute(&mut conn)
            .await
    );

    let json: UserShowPublicResponse = anon.get("/api/v1/users/fOObAr").await.good();
    assert_eq!(
        "I was second, I took the foobar username on github",
        json.user.name.unwrap()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn user_without_github_account() {
    let (app, anon) = TestApp::init().empty().await;
    let mut conn = app.db_conn().await;

    let new_user = NewUser::builder()
        // The gh_id column will eventually be removed; there are currently records in production
        // that have `-1` for their `gh_id` because the associated GitHub accounts have been deleted
        .gh_id(-1)
        .gh_login("foobar")
        .name("I deleted my github account")
        .gh_encrypted_token(&[])
        .build();
    new_user.insert(&mut conn).await.unwrap();
    // This user doesn't have a linked record in `oauth_github`

    // The crates.io username still exists
    let json: UserShowPublicResponse = anon.get("/api/v1/users/fOObAr").await.good();
    assert_eq!("I deleted my github account", json.user.name.unwrap());
}
