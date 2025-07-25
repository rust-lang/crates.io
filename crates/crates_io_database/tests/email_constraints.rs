//! Tests to verify that the SQL constraints on the `emails` table are enforced correctly.

use crates_io_database::models::{Email, NewEmail, NewUser};
use crates_io_database::schema::{emails, users};
use crates_io_test_db::TestDatabase;
use diesel::prelude::*;
use diesel::result::Error;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use insta::assert_debug_snapshot;

const MAX_EMAIL_COUNT: i32 = 32;

#[derive(Debug)]
#[allow(dead_code)]
/// A snapshot of the email data used for testing.
/// This struct is used to compare the results of database operations against expected values.
/// We can't use `Email` directly because it contains date/time fields that would change each time.
struct EmailSnapshot {
    id: i32,
    user_id: i32,
    email: String,
    primary: bool,
}
impl From<Email> for EmailSnapshot {
    fn from(email: Email) -> Self {
        EmailSnapshot {
            id: email.id,
            user_id: email.user_id,
            email: email.email,
            primary: email.primary,
        }
    }
}

// Insert a test user into the database and return its ID.
async fn insert_test_user(conn: &mut AsyncPgConnection) -> i32 {
    let user_count = users::table.count().get_result::<i64>(conn).await.unwrap();
    let user = NewUser::builder()
        .name(&format!("testuser{}", user_count + 1))
        .gh_id(user_count as i32 + 1)
        .gh_login(&format!("testuser{}", user_count + 1))
        .gh_access_token("token")
        .build()
        .insert(conn)
        .await
        .unwrap();
    user.id
}

// Insert a basic primary email for a user.
async fn insert_static_primary_email(
    conn: &mut AsyncPgConnection,
    user_id: i32,
) -> Result<Email, Error> {
    NewEmail::builder()
        .user_id(user_id)
        .email("primary@example.com")
        .primary(true)
        .build()
        .insert(conn)
        .await
}

// Insert a basic secondary email for a user.
async fn insert_static_secondary_email(
    conn: &mut AsyncPgConnection,
    user_id: i32,
) -> Result<Email, Error> {
    NewEmail::builder()
        .user_id(user_id)
        .email("secondary@example.com")
        .primary(false)
        .build()
        .insert(conn)
        .await
}

#[tokio::test]
// Add a primary email address to the database.
async fn create_primary_email() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let user_id: i32 = insert_test_user(&mut conn).await;

    let result = insert_static_primary_email(&mut conn, user_id)
        .await
        .map(|email| EmailSnapshot::from(email));

    assert_debug_snapshot!(result);
}

#[tokio::test]
// Attempt to create a secondary email address without a primary already present, which should fail.
// This tests the `verify_exactly_one_primary_email` trigger.
async fn create_secondary_email_without_primary() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let user_id: i32 = insert_test_user(&mut conn).await;

    let result = insert_static_secondary_email(&mut conn, user_id).await;

    assert_debug_snapshot!(result);
}

#[tokio::test]
// Attempt to delete the only email address for a user, which should succeed.
// This tests the `prevent_primary_email_deletion` trigger.
async fn delete_only_email() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let user_id: i32 = insert_test_user(&mut conn).await;

    let email = insert_static_primary_email(&mut conn, user_id)
        .await
        .expect("failed to insert primary email");

    let result = diesel::delete(emails::table.find(email.id))
        .execute(&mut conn)
        .await;

    assert_debug_snapshot!(result);
}

#[tokio::test]
// Add a secondary email address to the database.
async fn create_secondary_email() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let user_id: i32 = insert_test_user(&mut conn).await;

    let primary = insert_static_primary_email(&mut conn, user_id)
        .await
        .map(|email| EmailSnapshot::from(email));

    let secondary = insert_static_secondary_email(&mut conn, user_id)
        .await
        .map(|email| EmailSnapshot::from(email));

    assert_debug_snapshot!((primary, secondary));
}

#[tokio::test]
// Attempt to delete a secondary email address, which should succeed.
// This tests the `prevent_primary_email_deletion` trigger.
async fn delete_secondary_email() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let user_id: i32 = insert_test_user(&mut conn).await;

    let _primary = insert_static_primary_email(&mut conn, user_id)
        .await
        .expect("failed to insert primary email");

    let secondary = insert_static_secondary_email(&mut conn, user_id)
        .await
        .expect("failed to insert secondary email");

    let result = diesel::delete(emails::table.find(secondary.id))
        .execute(&mut conn)
        .await;

    assert_debug_snapshot!(result);
}

#[tokio::test]
// Attempt to delete a primary email address when a secondary email exists, which should fail.
// This tests the `prevent_primary_email_deletion` trigger.
async fn delete_primary_email_with_secondary() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let user_id: i32 = insert_test_user(&mut conn).await;

    let primary = insert_static_primary_email(&mut conn, user_id)
        .await
        .expect("failed to insert primary email");

    let _secondary = insert_static_secondary_email(&mut conn, user_id)
        .await
        .expect("failed to insert secondary email");

    let result = diesel::delete(emails::table.find(primary.id))
        .execute(&mut conn)
        .await;

    assert_debug_snapshot!(result);
}

#[tokio::test]
// Attempt to add a secondary email address with the same email as the primary, which should fail.
// This tests the `unique_user_email` constraint.
async fn create_secondary_email_with_same_email_as_primary() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let user_id: i32 = insert_test_user(&mut conn).await;

    let primary = insert_static_primary_email(&mut conn, user_id)
        .await
        .map(|email| EmailSnapshot::from(email))
        .expect("failed to insert primary email");

    let secondary = NewEmail::builder()
        .user_id(user_id)
        .email(&primary.email)
        .primary(false)
        .build()
        .insert(&mut conn)
        .await
        .map(|email| EmailSnapshot::from(email));

    assert_debug_snapshot!((primary, secondary));
}

#[tokio::test]
// Attempt to create more than the maximum allowed emails for a user, which should fail.
// This tests the `enforce_max_emails_per_user` trigger.
async fn create_too_many_emails() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let user_id: i32 = insert_test_user(&mut conn).await;

    let mut errors = Vec::new();
    for i in 0..MAX_EMAIL_COUNT + 2 {
        let result = NewEmail::builder()
            .user_id(user_id)
            .email(&format!("me+{}@example.com", i))
            .primary(i == 0)
            .build()
            .insert(&mut conn)
            .await
            .map(|email| EmailSnapshot::from(email));

        if let Err(err) = result {
            errors.push(err);
        }
    }

    assert_debug_snapshot!(errors);
}

#[tokio::test]
// Attempt to add the same email address to two users, which should succeed.
async fn create_same_email_for_different_users() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let user_id_1: i32 = insert_test_user(&mut conn).await;
    let user_id_2: i32 = insert_test_user(&mut conn).await;

    let first = insert_static_primary_email(&mut conn, user_id_1)
        .await
        .map(|email| EmailSnapshot::from(email));

    let second = insert_static_primary_email(&mut conn, user_id_2)
        .await
        .map(|email| EmailSnapshot::from(email));

    assert_debug_snapshot!((first, second));
}

#[tokio::test]
// Create a primary email, a secondary email, and then promote the secondary email to primary.
// This tests the `promote_email_to_primary` function and the `unique_primary_email_per_user` constraint.
async fn promote_secondary_to_primary() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let user_id: i32 = insert_test_user(&mut conn).await;

    let _primary = insert_static_primary_email(&mut conn, user_id)
        .await
        .expect("failed to insert primary email");

    let secondary = insert_static_secondary_email(&mut conn, user_id)
        .await
        .expect("failed to insert secondary email");

    diesel::sql_query("SELECT promote_email_to_primary($1)")
        .bind::<diesel::sql_types::Integer, _>(secondary.id)
        .execute(&mut conn)
        .await
        .expect("failed to promote secondary email to primary");

    // Query both emails to verify that the primary flag has been updated correctly for both.
    let result = emails::table
        .select((emails::id, emails::email, emails::primary))
        .load::<(i32, String, bool)>(&mut conn)
        .await;

    assert_debug_snapshot!(result);
}

#[tokio::test]
// Attempt to demote a primary email to secondary without assigning another primary, which should fail.
// This tests the `verify_exactly_one_primary_email` trigger.
async fn demote_primary_without_new_primary() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let user_id: i32 = insert_test_user(&mut conn).await;

    let primary = insert_static_primary_email(&mut conn, user_id)
        .await
        .expect("failed to insert primary email");

    let result = diesel::update(emails::table.find(primary.id))
        .set(emails::primary.eq(false))
        .execute(&mut conn)
        .await;

    assert_debug_snapshot!(result);
}

#[tokio::test]
// Attempt to create a primary email when one already exists for the user, which should fail.
// This tests the `unique_primary_email_per_user` constraint.
async fn create_primary_email_when_one_exists() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;
    let user_id: i32 = insert_test_user(&mut conn).await;

    let first = insert_static_primary_email(&mut conn, user_id)
        .await
        .map(|email| EmailSnapshot::from(email));

    let second = NewEmail::builder()
        .user_id(user_id)
        .email("me+2@example.com")
        .primary(true)
        .build()
        .insert(&mut conn)
        .await
        .map(|email| EmailSnapshot::from(email));

    assert_debug_snapshot!((first, second));
}
