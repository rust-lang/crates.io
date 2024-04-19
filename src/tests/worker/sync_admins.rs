use crate::util::TestApp;
use crates_io::schema::{emails, users};
use crates_io::team_repo::{MockTeamRepo, Permission, Person};
use crates_io::worker::jobs::SyncAdmins;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel::{PgConnection, QueryResult, RunQueryDsl};
use insta::assert_debug_snapshot;
use regex::Regex;

#[tokio::test(flavor = "multi_thread")]
async fn test_sync_admins_job() {
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

    let (app, _) = TestApp::full().with_team_repo(team_repo).empty();

    app.db(|conn| create_user("existing-admin", 1, true, conn).unwrap());
    app.db(|conn| create_user("obsolete-admin", 2, true, conn).unwrap());
    app.db(|conn| create_user("new-admin", 3, false, conn).unwrap());
    app.db(|conn| create_user("unrelated-user", 42, false, conn).unwrap());

    let admins = app.db(|conn| get_admins(conn).unwrap());
    let expected_admins = vec![("existing-admin".into(), 1), ("obsolete-admin".into(), 2)];
    assert_eq!(admins, expected_admins);

    app.db(|conn| SyncAdmins.enqueue(conn).unwrap());
    app.run_pending_background_jobs().await;

    let admins = app.db(|conn| get_admins(conn).unwrap());
    let expected_admins = vec![("existing-admin".into(), 1), ("new-admin".into(), 3)];
    assert_eq!(admins, expected_admins);

    let email_header_regex = Regex::new(r"(Message-ID|Date): [^\r\n]+\r\n").unwrap();
    let emails = app.as_inner().emails.mails_in_memory().unwrap();
    let emails = emails
        .iter()
        .map(|(_, email)| email_header_regex.replace_all(email, ""))
        .collect::<Vec<_>>();

    assert_debug_snapshot!(emails);

    // Run the job again to verify that no new emails are sent
    // for `new-admin-without-account`.
    app.db(|conn| SyncAdmins.enqueue(conn).unwrap());
    app.run_pending_background_jobs().await;

    let emails = app.as_inner().emails.mails_in_memory().unwrap();
    assert_eq!(emails.len(), 2);
}

fn mock_permission(people: Vec<Person>) -> Permission {
    Permission { people }
}

fn mock_person(name: impl Into<String>, github_id: i32) -> Person {
    let name = name.into();
    let github = name.clone();
    Person {
        name,
        github,
        github_id,
    }
}

fn create_user(name: &str, gh_id: i32, is_admin: bool, conn: &mut PgConnection) -> QueryResult<()> {
    let user_id = diesel::insert_into(users::table)
        .values((
            users::name.eq(name),
            users::gh_login.eq(name),
            users::gh_id.eq(gh_id),
            users::gh_access_token.eq("some random token"),
            users::is_admin.eq(is_admin),
        ))
        .returning(users::id)
        .get_result::<i32>(conn)?;

    diesel::insert_into(emails::table)
        .values((
            emails::user_id.eq(user_id),
            emails::email.eq(format!("{}@crates.io", name)),
            emails::verified.eq(true),
        ))
        .execute(conn)?;

    Ok(())
}

fn get_admins(conn: &mut PgConnection) -> QueryResult<Vec<(String, i32)>> {
    users::table
        .select((users::gh_login, users::gh_id))
        .filter(users::is_admin.eq(true))
        .order(users::gh_id.asc())
        .get_results(conn)
}
