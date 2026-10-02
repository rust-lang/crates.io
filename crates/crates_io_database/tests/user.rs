use chrono::{Duration, Utc};
use claims::{assert_err_eq, assert_lt, assert_none, assert_some};
use crates_io_database::models::{NewUser, PublicUser, User, users_by_username};
use crates_io_database::schema::users;
use crates_io_test_db::TestDatabase;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[tokio::test]
async fn find_latest_user_by_canonical_username() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let first_id = NewUser::builder()
        .gh_id(1)
        .gh_login("foo-bar")
        .username("foo-bar")
        .build()
        .insert(&conn)
        .await
        .unwrap();
    let second_id = NewUser::builder()
        .gh_id(2)
        .gh_login("FOO_BAR")
        .username("FOO_BAR")
        .build()
        .insert(&conn)
        .await
        .unwrap();

    assert!(second_id > first_id);

    let user_id: i32 = users_by_username("Foo-Bar")
        .select(users::id)
        .first(&mut conn)
        .await
        .unwrap();

    assert_eq!(user_id, second_id);
}

/// Public lookup works without linked accounts and rejects unknown IDs.
#[tokio::test]
async fn find_public_user_by_id() {
    let test_db = TestDatabase::new();
    let conn = test_db.async_connect().await;
    let user = NewUser::builder()
        .gh_id(1)
        .gh_login("github-user")
        .username("crates-user")
        .build();
    let id = user.insert(&conn).await.unwrap();

    let user = PublicUser::find(&conn, id).await.unwrap();
    assert_eq!(user.id, id);
    assert_eq!(user.username, "crates-user");
    assert!(!user.github_username_matches);

    let missing = PublicUser::find(&conn, 0).await;
    assert_err_eq!(missing, diesel::result::Error::NotFound);
}

#[tokio::test]
async fn is_locked() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let user = NewUser::builder()
        .gh_id(1)
        .gh_login("github-user")
        .username("crates-user")
        .build();
    let id = user.insert(&conn).await.unwrap();

    // A newly created user is not locked.
    let user = User::find(&conn, id).await.unwrap();
    assert_none!(user.is_locked());

    // A user with a reason and no expiry is locked indefinitely.
    diesel::update(users::table)
        .set(users::account_lock_reason.eq("locked indefinitely"))
        .filter(users::id.eq(id))
        .execute(&mut conn)
        .await
        .unwrap();
    let user = User::find(&conn, id).await.unwrap();
    let (reason, until) = assert_some!(user.is_locked());
    assert_eq!(reason, "locked indefinitely");
    assert_none!(until);

    // A user with a reason and an expiry in the future is also locked.
    let set_until = Utc::now() + Duration::days(365);
    diesel::update(users::table)
        .set((
            users::account_lock_reason.eq("locked definitely"),
            users::account_lock_until.eq(Some(set_until)),
        ))
        .filter(users::id.eq(id))
        .execute(&mut conn)
        .await
        .unwrap();
    let user = User::find(&conn, id).await.unwrap();
    let (reason, until) = assert_some!(user.is_locked());
    assert_eq!(reason, "locked definitely");

    // Note that the different precision with which PostgreSQL stores timestamps
    // compared to chrono means that we can't do a straight equality check for
    // `until`. Instead, we'll check that the difference is < 1 second, which is
    // more than enough wiggle room.
    let until = assert_some!(until);
    assert_lt!(until - set_until, Duration::seconds(1));

    // A user with a reason and an expiry in the past is not locked.
    let set_until = Utc::now() - Duration::days(365);
    diesel::update(users::table)
        .set((
            users::account_lock_reason.eq("locked in the past"),
            users::account_lock_until.eq(Some(set_until)),
        ))
        .filter(users::id.eq(id))
        .execute(&mut conn)
        .await
        .unwrap();
    let user = User::find(&conn, id).await.unwrap();
    assert_none!(user.is_locked());
}
