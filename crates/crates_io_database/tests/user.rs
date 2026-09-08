use claims::assert_err_eq;
use crates_io_database::models::{NewUser, PublicUser, users_by_username};
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
