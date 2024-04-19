use crate::util::{RequestHelper, TestApp};
use crates_io::models::NewUser;
use crates_io::views::EncodablePublicUser;

#[derive(Deserialize)]
pub struct UserShowPublicResponse {
    pub user: EncodablePublicUser,
}

#[tokio::test(flavor = "multi_thread")]
async fn show() {
    let (app, anon, _) = TestApp::init().with_user();
    app.db_new_user("Bar");

    let json: UserShowPublicResponse = anon.async_get("/api/v1/users/foo").await.good();
    assert_eq!(json.user.login, "foo");

    let json: UserShowPublicResponse = anon.async_get("/api/v1/users/bAr").await.good();
    assert_eq!(json.user.login, "Bar");
    assert_eq!(json.user.url, "https://github.com/Bar");
}

#[tokio::test(flavor = "multi_thread")]
async fn show_latest_user_case_insensitively() {
    let (app, anon) = TestApp::init().empty();

    app.db(|conn| {
        // Please do not delete or modify the setup of this test in order to get it to pass.
        // This setup mimics how GitHub works. If someone abandons a GitHub account, the username is
        // available for anyone to take. We need to support having multiple user accounts
        // with the same gh_login in crates.io. `gh_id` is stable across renames, so that field
        // should be used for uniquely identifying GitHub accounts whenever possible. For the
        // crates.io/user/:username pages, the best we can do is show the last crates.io account
        // created with that username.
        assert_ok!(NewUser::new(
            1,
            "foobar",
            Some("I was first then deleted my github account"),
            None,
            "bar"
        )
        .create_or_update(None, &app.as_inner().emails, conn));
        assert_ok!(NewUser::new(
            2,
            "FOOBAR",
            Some("I was second, I took the foobar username on github"),
            None,
            "bar"
        )
        .create_or_update(None, &app.as_inner().emails, conn));
    });

    let json: UserShowPublicResponse = anon.async_get("/api/v1/users/fOObAr").await.good();
    assert_eq!(
        "I was second, I took the foobar username on github",
        json.user.name.unwrap()
    );
}
