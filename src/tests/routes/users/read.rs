use crate::models::NewUser;
use crate::schema::users;
use crate::tests::util::{RequestHelper, TestApp};
use crate::views::EncodablePublicUser;
use claims::assert_ok;
use diesel_async::RunQueryDsl;
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
