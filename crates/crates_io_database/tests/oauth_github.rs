use crates_io_database::models::{NewOauthGithub, NewUser, OauthGithub};
use crates_io_test_db::TestDatabase;
use diesel::OptionalExtension;
use diesel_async::AsyncPgConnection;

/// Creates a user and linked GitHub account.
async fn insert_user(conn: &AsyncPgConnection, username: &str, gh_id: i32, login: &str) -> i32 {
    let user_id = NewUser::builder()
        .username(username)
        .gh_login(login)
        .gh_id(gh_id)
        .build()
        .insert(conn)
        .await
        .unwrap();

    NewOauthGithub::builder()
        .user_id(user_id)
        .account_id(i64::from(gh_id))
        .login(login)
        .encrypted_token(&[])
        .build()
        .insert(conn)
        .await
        .unwrap();

    user_id
}

/// GitHub lookup uses the OAuth login and preserves the distinction between separators.
#[tokio::test]
async fn oauth_github_find_by_login() {
    let test_db = TestDatabase::new();
    let conn = test_db.async_connect().await;
    let user_id = insert_user(&conn, "crates-user", 1, "GitHub-User").await;

    for login in ["github-user", "GitHub-User", "GITHUB-USER"] {
        let account = OauthGithub::find_by_login(&conn, login).await.unwrap();
        assert_eq!(account.user_id, user_id);
        assert_eq!(account.account_id, 1);
        assert_eq!(account.login, "GitHub-User");
    }

    for login in ["github_user", "missing", "crates-user"] {
        let account = OauthGithub::find_by_login(&conn, login).await.optional();
        assert!(account.unwrap().is_none(), "unexpected match for {login}");
    }
}

/// Reused logins select the highest GitHub account ID.
#[tokio::test]
async fn oauth_github_find_by_login_uses_highest_account_id() {
    let test_db = TestDatabase::new();
    let conn = test_db.async_connect().await;
    let user_id = insert_user(&conn, "newer-user", 2, "shared").await;
    insert_user(&conn, "older-user", 1, "SHARED").await;

    let account = OauthGithub::find_by_login(&conn, "shared").await.unwrap();
    assert_eq!(account.account_id, 2);
    assert_eq!(account.user_id, user_id);
}
