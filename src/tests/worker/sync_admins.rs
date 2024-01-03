use crate::util::TestApp;
use crates_io::schema::users;
use crates_io::team_repo::{Member, MockTeamRepo, Team};
use crates_io::worker::jobs::SyncAdmins;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel::{PgConnection, QueryResult, RunQueryDsl};

#[test]
fn test_sync_admins_job() {
    let mock_response = mock_team(
        "crates-io",
        vec![
            mock_member("existing-admin", 1),
            mock_member("new-admin", 3),
        ],
    );

    let mut team_repo = MockTeamRepo::new();
    team_repo
        .expect_get_team()
        .with(mockall::predicate::eq("crates-io-admins"))
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
    app.run_pending_background_jobs();

    let admins = app.db(|conn| get_admins(conn).unwrap());
    let expected_admins = vec![("existing-admin".into(), 1), ("new-admin".into(), 3)];
    assert_eq!(admins, expected_admins);
}

fn mock_team(name: impl Into<String>, members: Vec<Member>) -> Team {
    Team {
        name: name.into(),
        kind: "marker-team".to_string(),
        members,
    }
}

fn mock_member(name: impl Into<String>, github_id: i32) -> Member {
    let name = name.into();
    let github = name.clone();
    Member {
        name,
        github,
        github_id,
        is_lead: false,
    }
}

fn create_user(name: &str, gh_id: i32, is_admin: bool, conn: &mut PgConnection) -> QueryResult<()> {
    diesel::insert_into(users::table)
        .values((
            users::name.eq(name),
            users::gh_login.eq(name),
            users::gh_id.eq(gh_id),
            users::gh_access_token.eq("some random token"),
            users::is_admin.eq(is_admin),
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
