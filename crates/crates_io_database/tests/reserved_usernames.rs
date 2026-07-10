//! Tests for the database trigger that blocks reserved usernames. Each test
//! seeds the `reserved_usernames` table and then drives `users` inserts/updates
//! directly, asserting the trigger rejects the conflicting writes.

use claims::assert_err;
use crates_io_database::schema::{reserved_usernames, users};
use crates_io_test_db::TestDatabase;
use crates_io_test_utils::builders::UserBuilder;
use diesel::prelude::*;
use diesel::result::QueryResult;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

async fn reserve_username(conn: &mut AsyncPgConnection, username: &str) {
    diesel::insert_into(reserved_usernames::table)
        .values(reserved_usernames::username.eq(username))
        .execute(conn)
        .await
        .unwrap();
}

async fn insert_user(conn: &AsyncPgConnection, username: &str) -> QueryResult<i32> {
    UserBuilder::new()
        .with_username(username)
        .new_user()
        .insert(conn)
        .await
}

#[tokio::test]
async fn insert_with_reserved_username_is_rejected() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    reserve_username(&mut conn, "admin").await;

    insert_user(&conn, "alice").await.unwrap();

    let error = assert_err!(insert_user(&conn, "admin").await);
    insta::assert_snapshot!(error, @"cannot create user with reserved username");
}

#[tokio::test]
async fn update_to_reserved_username_is_rejected() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    reserve_username(&mut conn, "admin").await;
    let user_id = insert_user(&conn, "bob").await.unwrap();

    let result = diesel::update(users::table.find(user_id))
        .set(users::username.eq("admin"))
        .execute(&mut conn)
        .await;

    insta::assert_snapshot!(assert_err!(result), @"cannot create user with reserved username");
}
