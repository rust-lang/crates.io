use crate::util::TestApp;
use crates_io::schema::{emails, oauth_github, users};
use crates_io::worker::jobs::SyncAdmins;
use crates_io_team_repo::{MockTeamRepo, Permission, Person};
use crates_io_worker::BackgroundJob;
use diesel::QueryResult;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn test_sync_admins_job() -> anyhow::Result<()> {
    let mock_response = mock_permission(vec![
        mock_person("existing-admin", 1),
        mock_person("new-admin", 3),
        mock_person("new-admin-without-account", 4),
    ]);

    let mut team_repo = MockTeamRepo::new();
    team_repo
        .expect_get_permission()
        .with(mockall::predicate::eq("crates_io_admin"))
        .returning(move |_| Ok(mock_response.clone()));

    let (app, _) = TestApp::full().with_team_repo(team_repo).empty().await;
    let mut conn = app.db_conn().await;

    create_user("existing-admin", 1, true, &mut conn).await?;
    create_user("obsolete-admin", 2, true, &mut conn).await?;
    create_user("new-admin", 3, false, &mut conn).await?;

    // When we allow for accounts to be associated with services other than GitHub, this accounts
    // for the case where:
    // - User was an admin via GitHub (for the foreseeable future, admins will still need GitHub)
    // - User adds a login via some other service and removes their GitHub account association
    // - User leaves the crates.io admin team
    // - Sync admins runs
    // The user should  be an admin before syncing but not after syncing.
    create_user("obsolete-admin-without-github", 5, true, &mut conn).await?;
    delete_oauth_github_from_user(5, &mut conn).await?;

    create_user("unrelated-user", 42, false, &mut conn).await?;

    let admins = get_admins(&mut conn).await?;
    let expected_admins: Vec<String> = vec![
        "existing-admin".into(),
        "obsolete-admin".into(),
        "obsolete-admin-without-github".into(),
    ];
    assert_eq!(admins, expected_admins);

    SyncAdmins.enqueue(&conn).await?;
    app.run_pending_background_jobs().await;

    let admins = get_admins(&mut conn).await?;
    let expected_admins: Vec<String> = vec!["existing-admin".into(), "new-admin".into()];
    assert_eq!(admins, expected_admins);

    assert_snapshot!(app.emails_snapshot().await);

    // Run the job again to verify that no new emails are sent
    // for `new-admin-without-account`.
    SyncAdmins.enqueue(&conn).await?;
    app.run_pending_background_jobs().await;

    assert_eq!(app.emails().await.len(), 3);

    Ok(())
}

fn mock_permission(people: Vec<Person>) -> Permission {
    Permission { people }
}

fn mock_person(name: impl Into<String>, github_id: i64) -> Person {
    let name = name.into();
    let github = name.clone();
    Person {
        name,
        github,
        github_id,
    }
}

async fn create_user(
    name: &str,
    account_id: i64,
    is_admin: bool,
    conn: &mut AsyncPgConnection,
) -> QueryResult<()> {
    let user_id = diesel::insert_into(users::table)
        .values((
            users::name.eq(name),
            users::gh_login.eq(name),
            users::gh_id.eq(account_id as i32),
            users::gh_encrypted_token.eq(&[]),
            users::is_admin.eq(is_admin),
        ))
        .returning(users::id)
        .get_result::<i32>(conn)
        .await?;

    diesel::insert_into(oauth_github::table)
        .values((
            oauth_github::user_id.eq(user_id),
            oauth_github::login.eq(name),
            oauth_github::account_id.eq(account_id),
            oauth_github::encrypted_token.eq(&[]),
        ))
        .execute(conn)
        .await?;

    diesel::insert_into(emails::table)
        .values((
            emails::user_id.eq(user_id),
            emails::email.eq(format!("{name}@crates.io")),
            emails::verified.eq(true),
        ))
        .execute(conn)
        .await?;

    Ok(())
}

async fn delete_oauth_github_from_user(
    account_id: i64,
    conn: &mut AsyncPgConnection,
) -> QueryResult<()> {
    diesel::delete(oauth_github::table.filter(oauth_github::account_id.eq(account_id)))
        .execute(conn)
        .await?;
    Ok(())
}

/// Regression test: sync_admins matches users via oauth_github.account_id,
/// not via the legacy users.gh_id column directly. This verifies the job
/// still works correctly after the Tier 1 identity read cutover.
///
/// The scenario: A user has a matching oauth_github.account_id but no entry in
/// users.gh_id (or mismatched). The sync_admins job should match and grant admin
/// access via the oauth_github join, proving it does not rely on the legacy
/// users.gh_id path.
#[tokio::test(flavor = "multi_thread")]
async fn sync_admins_sets_admin_via_oauth_github_account_id() -> anyhow::Result<()> {
    let admin_github_id = 100i64;

    let mock_response = mock_permission(vec![
        mock_person("admin-user", admin_github_id),
    ]);

    let mut team_repo = MockTeamRepo::new();
    team_repo
        .expect_get_permission()
        .with(mockall::predicate::eq("crates_io_admin"))
        .returning(move |_| Ok(mock_response.clone()));

    let (app, _) = TestApp::full().with_team_repo(team_repo).empty().await;
    let mut conn = app.db_conn().await;

    // Create a user with a distinct users.gh_id (9999) but oauth_github.account_id
    // set to the admin ID (100). This tests that the matching happens via the
    // oauth_github join, not through users.gh_id.
    let user_id = diesel::insert_into(users::table)
        .values((
            users::name.eq("admin-user"),
            users::gh_login.eq("admin-user"),
            users::gh_id.eq(9999i32),  // Deliberately different from admin_github_id
            users::gh_encrypted_token.eq(&[]),
            users::is_admin.eq(false),
        ))
        .returning(users::id)
        .get_result::<i32>(&mut conn)
        .await?;

    // The oauth_github record has the matching admin ID
    diesel::insert_into(oauth_github::table)
        .values((
            oauth_github::user_id.eq(user_id),
            oauth_github::login.eq("admin-user"),
            oauth_github::account_id.eq(admin_github_id),  // Matches the admin list
            oauth_github::encrypted_token.eq(&[]),
        ))
        .execute(&mut conn)
        .await?;

    diesel::insert_into(emails::table)
        .values((
            emails::user_id.eq(user_id),
            emails::email.eq("admin-user@crates.io"),
            emails::verified.eq(true),
        ))
        .execute(&mut conn)
        .await?;

    // Verify initial state: not an admin
    let is_admin_before = users::table
        .select(users::is_admin)
        .filter(users::gh_login.eq("admin-user"))
        .get_result::<bool>(&mut conn)
        .await?;
    assert!(!is_admin_before, "user should start as non-admin");

    // Run sync_admins
    SyncAdmins.enqueue(&conn).await?;
    app.run_pending_background_jobs().await;

    // After sync: the user should be admin because oauth_github.account_id matched,
    // even though their users.gh_id (9999) does not match admin_github_id (100).
    // This proves sync_admins uses the oauth_github join, not legacy gh_id matching.
    let is_admin_after = users::table
        .select(users::is_admin)
        .filter(users::gh_login.eq("admin-user"))
        .get_result::<bool>(&mut conn)
        .await?;
    assert!(is_admin_after,
        "user with matching oauth_github.account_id should become admin, \
        proving sync_admins uses oauth_github join (not legacy gh_id fallback)");

    Ok(())
}

async fn get_admins(conn: &mut AsyncPgConnection) -> QueryResult<Vec<String>> {
    users::table
        .select(users::gh_login)
        .filter(users::is_admin.eq(true))
        .order(users::id.asc())
        .get_results(conn)
        .await
}
