use chrono::{TimeDelta, Utc};
use crates_io_database::models::{NewOauthGithub, NewUser, OauthGithub};
use crates_io_database::schema::oauth_github;
use crates_io_test_db::TestDatabase;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

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

/// Sync batches select the least recently synced accounts up to the requested limit.
#[tokio::test]
async fn oauth_github_sync_batch() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let now = Utc::now();

    for (id, days_since_sync) in [(1, 1), (2, 3), (3, 2)] {
        let login = format!("user-{id}");
        insert_user(&conn, &login, id, &login).await;
        diesel::update(oauth_github::table.find(i64::from(id)))
            .set(oauth_github::last_sync.eq(now - TimeDelta::days(days_since_sync)))
            .execute(&mut conn)
            .await
            .unwrap();
    }

    for (batch_size, expected_ids) in [(0, vec![]), (2, vec![2, 3]), (10, vec![2, 3, 1])] {
        let accounts = OauthGithub::sync_batch(&conn, batch_size).await.unwrap();
        let ids: Vec<_> = accounts.iter().map(|account| account.account_id).collect();
        assert_eq!(ids, expected_ids, "batch size {batch_size}");
    }
}

/// Older GitHub accounts with conflicting non-enterprise logins take priority regardless of sync
/// time.
#[tokio::test]
async fn oauth_github_sync_batch_prioritizes_conflicting_logins() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let now = Utc::now();

    for (id, login, days_since_sync) in [
        (1, "shared-name", 1),
        (2, "SHARED-NAME", 2),
        (3, "Shared-Name", 5),
        (4, "unrelated", 6),
        (5, "shared_name", 7),
        (6, "SHARED_NAME", 4),
    ] {
        insert_user(&conn, &format!("user-{id}"), id, login).await;
        diesel::update(oauth_github::table.find(i64::from(id)))
            .set(oauth_github::last_sync.eq(now - TimeDelta::days(days_since_sync)))
            .execute(&mut conn)
            .await
            .unwrap();
    }

    for (batch_size, expected_ids) in [
        (0, vec![]),
        (1, vec![2]),
        (3, vec![2, 1, 5]),
        (10, vec![2, 1, 5, 4, 3, 6]),
    ] {
        let accounts = OauthGithub::sync_batch(&conn, batch_size).await.unwrap();
        let ids: Vec<_> = accounts.iter().map(|account| account.account_id).collect();
        assert_eq!(ids, expected_ids, "batch size {batch_size}");
    }
}
