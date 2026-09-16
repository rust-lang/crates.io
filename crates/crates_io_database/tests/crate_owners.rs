use crates_io_database::models::{Crate, CrateOwner, NewCrate, NewUser, OwnerKind};
use crates_io_database::schema::{crate_owners, teams};
use crates_io_test_db::TestDatabase;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

/// Batch removal matches complete owner identities within one crate.
#[tokio::test]
async fn remove_owners_by_identity() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let user1 = insert_user(&mut conn, "first", 101).await;
    let user2 = insert_user(&mut conn, "second", 102).await;
    let team1 = insert_team(&mut conn, "first", 201).await;
    let team2 = insert_team(&mut conn, "second", 202).await;
    let team3 = insert_team(&mut conn, "non-owner", 203).await;
    let crate1 = insert_crate(&mut conn, "foo", user1).await;
    let crate2 = insert_crate(&mut conn, "bar", user1).await;

    insert_crate_owner(&conn, crate1.id, OwnerKind::User, user1, user1).await;
    insert_crate_owner(&conn, crate1.id, OwnerKind::User, user2, user1).await;
    insert_crate_owner(&conn, crate1.id, OwnerKind::Team, team1, user1).await;
    insert_crate_owner(&conn, crate1.id, OwnerKind::Team, team2, user1).await;
    insert_crate_owner(&conn, crate2.id, OwnerKind::User, user1, user1).await;
    insert_crate_owner(&conn, crate2.id, OwnerKind::User, user2, user1).await;
    insert_crate_owner(&conn, crate2.id, OwnerKind::Team, team1, user1).await;
    insert_crate_owner(&conn, crate2.id, OwnerKind::Team, team2, user1).await;

    assert_eq!(crate1.remove_owners(&conn, &[]).await.unwrap(), 0);
    let selected = [
        (OwnerKind::User, user1),
        (OwnerKind::Team, team2),
        (OwnerKind::User, user1),
        (OwnerKind::Team, team3),
    ];
    assert_eq!(crate1.remove_owners(&conn, &selected).await.unwrap(), 2);
    assert_eq!(crate1.remove_owners(&conn, &selected).await.unwrap(), 0);

    let rows = OwnerRow::query()
        .order((
            crate_owners::crate_id,
            crate_owners::owner_kind,
            crate_owners::owner_id,
        ))
        .load(&mut conn)
        .await
        .unwrap();

    let rows: Vec<_> = rows
        .into_iter()
        .map(|row| (row.crate_id, row.owner_kind, row.owner_id, row.deleted))
        .collect();

    assert_eq!(
        rows,
        [
            (crate1.id, 0, user1, true),
            (crate1.id, 0, user2, false),
            (crate1.id, 1, team1, false),
            (crate1.id, 1, team2, true),
            (crate2.id, 0, user1, false),
            (crate2.id, 0, user2, false),
            (crate2.id, 1, team1, false),
            (crate2.id, 1, team2, false),
        ]
    );
}

/// Stored ownership identity and deletion state.
#[derive(Debug, HasQuery)]
#[diesel(table_name = crate_owners)]
struct OwnerRow {
    crate_id: i32,
    owner_kind: i32,
    owner_id: i32,
    deleted: bool,
}

/// Inserts a user and returns its assigned ID.
async fn insert_user(conn: &mut AsyncPgConnection, name: &str, gh_id: i32) -> i32 {
    let user = NewUser::builder()
        .username(name)
        .gh_login(name)
        .gh_id(gh_id)
        .build();
    user.insert(conn).await.unwrap()
}

/// Inserts a team and returns its assigned ID.
async fn insert_team(conn: &mut AsyncPgConnection, name: &str, github_id: i32) -> i32 {
    let login = format!("github:org:{name}");
    diesel::insert_into(teams::table)
        .values((
            teams::login.eq(login),
            teams::github_id.eq(github_id),
            teams::org_id.eq(1),
        ))
        .returning(teams::id)
        .get_result(conn)
        .await
        .unwrap()
}

/// Creates a crate owned by the given user.
async fn insert_crate(conn: &mut AsyncPgConnection, name: &str, user_id: i32) -> Crate {
    let krate = NewCrate {
        name,
        ..Default::default()
    };
    krate.create(conn, user_id).await.unwrap()
}

/// Inserts an ownership record for a user or team.
async fn insert_crate_owner(
    conn: &AsyncPgConnection,
    crate_id: i32,
    owner_kind: OwnerKind,
    owner_id: i32,
    created_by: i32,
) {
    let owner = CrateOwner {
        crate_id,
        owner_id,
        owner_kind,
        created_by,
        email_notifications: true,
    };
    owner.insert(conn).await.unwrap();
}
