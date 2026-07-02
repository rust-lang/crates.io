use crate::util::{RequestHelper, Response, TestApp};
use chrono::{DateTime, TimeDelta, Utc};
use claims::{assert_ge, assert_le, assert_ok};
use crates_io_database::models::{AbandonedUsername, NewAbandonedUsername, User};
use crates_io_database::schema::{abandoned_usernames, reserved_usernames};
use diesel::HasQuery;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;
use insta::assert_snapshot;
use serde_json::json;

mod publish_notifications;

pub trait MockEmailHelper: RequestHelper {
    // TODO: I don't like the name of this method or `update_email` on the `MockCookieUser` impl;
    // this is starting to look like a builder might help?
    // I want to explore alternative abstractions in any case.
    async fn update_email_more_control(&self, user_id: i32, email: Option<&str>) -> Response<()> {
        let body = json!({"user": { "email": email }});
        let url = format!("/api/v1/users/{user_id}");
        self.put(&url, body.to_string()).await
    }
}

impl MockEmailHelper for crate::util::MockCookieUser {}
impl MockEmailHelper for crate::util::MockAnonymousUser {}

impl crate::util::MockCookieUser {
    pub async fn update_email(&self, email: &str) {
        let model = self.as_model();
        let response = self.update_email_more_control(model.id, Some(email)).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
    }

    async fn request_username_update(&self, new_username: &str) -> Response<()> {
        let model = self.as_model();
        let body = json!({"user": { "username": new_username }});
        let url = format!("/api/v1/users/{}", model.id);
        self.put(&url, body.to_string()).await
    }
}

/// Given a crates.io user, check to make sure that the user
/// cannot add to the database an empty string or null as
/// their email. If an attempt is made, `update_user.rs` will
/// return an error indicating that an empty email cannot be
/// added.
///
/// This is checked on the frontend already, but I'd like to
/// make sure that a user cannot get around that and delete
/// their email by adding an empty string.
#[tokio::test(flavor = "multi_thread")]
async fn test_empty_email_not_added() {
    let (_app, _anon, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    let response = user.update_email_more_control(model.id, Some("")).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"empty email rejected"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ignore_empty() {
    let (_app, _anon, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    let url = format!("/api/v1/users/{}", model.id);
    let payload = json!({"user": {}});
    let response = user.put::<()>(&url, payload.to_string()).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_ignore_nulls() {
    let (_app, _anon, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    let url = format!("/api/v1/users/{}", model.id);
    let payload = json!({"user": { "email": null }});
    let response = user.put::<()>(&url, payload.to_string()).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"ok":true}"#);
}

/// Check to make sure that neither other signed in users nor anonymous users can edit another
/// user's email address.
///
/// If an attempt is made, the endpoint will return an error indicating that the current user
/// does not match the requested user.
#[tokio::test(flavor = "multi_thread")]
async fn test_other_users_cannot_change_my_email() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let another_user = app.db_new_user("not_me").await;
    let another_user_model = another_user.as_model();

    let response = user
        .update_email_more_control(
            another_user_model.id,
            Some("pineapple@pineapples.pineapple"),
        )
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"current user does not match requested user"}]}"#);

    let response = anon
        .update_email_more_control(
            another_user_model.id,
            Some("pineapple@pineapples.pineapple"),
        )
        .await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_email_address() {
    let (_app, _, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    let response = user.update_email_more_control(model.id, Some("foo")).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid email address"}]}"#);
}

/// Runs what should be a successful rename and ensures that the
/// expected DB tables have been updated appropriately
#[tokio::test(flavor = "multi_thread")]
async fn test_change_username_happy_path() {
    let (app, _, user) = TestApp::init().with_user().await;
    let model = user.as_model();
    let mut conn = app.db_conn().await;

    let old_username = &model.username;
    let new_username = "new-foo_username1";

    // actuallly do the request
    let put_request_start = Utc::now(); // for checking the timestamps later
    let response: Response<()> = user.request_username_update(new_username).await;
    let put_request_end = Utc::now();

    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.json(), @r#"{"ok":true}"#);

    // ───── Postconditions ─────
    // 1. check that new abandoned username record was created
    let records: Vec<AbandonedUsername> = assert_ok!(
        AbandonedUsername::query()
            .filter(abandoned_usernames::username.eq(old_username))
            .load(&mut conn)
            .await
    );
    assert_eq!(records.len(), 1);
    let record = records.into_iter().next().unwrap();
    assert_eq!(record.previous_user_id, Some(model.id));
    assert_eq!(record.username, *old_username);
    assert_eq!(
        record.available_at,
        record.abandoned_at + TimeDelta::days(30) // TODO: don't hardcode this?
    );
    assert_eq!(record.adopted_at, model.created_at);
    assert_ge!(record.abandoned_at, put_request_start);
    assert_le!(record.abandoned_at, put_request_end);

    // 2. user record should have been updated
    let updated_user: User = assert_ok!(User::find(&conn, model.id).await);
    assert_eq!(updated_user.username, new_username);
    assert_eq!(
        updated_user.current_username_adopted_at,
        Some(record.abandoned_at)
    )
}

/// Check that invalid usernames are rejected
#[tokio::test(flavor = "multi_thread")]
async fn test_reject_prohibited_username_changes() {
    let (app, _, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let initial_model = User::find(&conn, user.as_model().id)
        .await
        .expect("initial user lookup");

    // populate various tables with values to test against
    let _squatter = app.db_new_user("desirable-username").await;
    diesel::insert_into(reserved_usernames::table)
        .values(reserved_usernames::username.eq("superadminuser"))
        .execute(&mut conn)
        .await
        .expect("db setup");
    diesel::insert_into(abandoned_usernames::table)
        .values(NewAbandonedUsername {
            username: "on-cooldown",
            previous_user_id: None,
            adopted_at: None,
            abandoned_at: &DateTime::parse_from_rfc3339("2000-01-01T12:00:00Z")
                .unwrap()
                .to_utc(),
            available_at: &DateTime::parse_from_rfc3339("3000-01-01T12:00:00Z")
                .unwrap()
                .to_utc(),
        })
        .execute(&mut conn)
        .await
        .expect("db setup");

    // ───── tests ─────
    // reject empty string "": it fails string validation
    let response = user.request_username_update("").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(
        response.json(),
        @r#"{"errors":[{"detail":"username cannot be empty"}]}"#);

    // reject "desirable-username": another user is using it
    let response = user.request_username_update("DeSiRaBlE_uSeRnAmE").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(
        response.json(),
        @r#"{"errors":[{"detail":"the username `DeSiRaBlE_uSeRnAmE` is not available"}]}"#
    );

    // reject "superadminuser": it is reserved
    let response = user.request_username_update("SuPeRaDmInUsEr").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(
        response.json(),
        @r#"{"errors":[{"detail":"the username `SuPeRaDmInUsEr` is reserved"}]}"#);

    // reject "on-cooldown": it is unavailable until the year 3000
    let response = user.request_username_update("On-CoOlDoWn").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(
        response.text(),
        @r#"{"errors":[{"detail":"The username `On-CoOlDoWn` was recently in use. This username will be available after 3000-01-01T12:00:00Z."}]}"#
    );

    // ───── Post-conditions ─────
    // user record should not have changed at all
    let final_model = assert_ok!(User::find(&conn, user.as_model().id).await);
    assert_eq!(initial_model.username, final_model.username);
    assert_eq!(
        initial_model.current_username_adopted_at,
        final_model.current_username_adopted_at
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_json() {
    let (_app, _anon, user) = TestApp::init().with_user().await;
    let model = user.as_model();

    let url = format!("/api/v1/users/{}", model.id);
    let response = user.put::<()>(&url, r#"{ "user": foo }"#).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to parse the request body as JSON: user: expected ident at line 1 column 12"}]}"#);
}
