use crate::{util::RequestHelper, TestApp};
use chrono::{Duration, NaiveDateTime, Utc};
use http::StatusCode;

const URL: &str = "/api/v1/me";
const LOCK_REASON: &str = "test lock reason";

fn lock_account(app: &TestApp, user_id: i32, until: Option<NaiveDateTime>) {
    app.db(|conn| {
        use crates_io::schema::users;
        use diesel::prelude::*;

        diesel::update(users::table)
            .set((
                users::account_lock_reason.eq(LOCK_REASON),
                users::account_lock_until.eq(until),
            ))
            .filter(users::id.eq(user_id))
            .execute(conn)
            .unwrap();
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn account_locked_indefinitely() {
    let (app, _anon, user) = TestApp::init().with_user();
    lock_account(&app, user.as_model().id, None);

    let response = user.async_get::<()>(URL).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let error_message = format!("This account is indefinitely locked. Reason: {LOCK_REASON}");
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": error_message }] })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn account_locked_with_future_expiry() {
    let until = Utc::now().naive_utc() + Duration::days(1);

    let (app, _anon, user) = TestApp::init().with_user();
    lock_account(&app, user.as_model().id, Some(until));

    let until = until.format("%Y-%m-%d at %H:%M:%S UTC");
    let response = user.async_get::<()>(URL).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let error_message = format!("This account is locked until {until}. Reason: {LOCK_REASON}");
    assert_eq!(
        response.json(),
        json!({ "errors": [{ "detail": error_message }] })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn expired_account_lock() {
    let until = Utc::now().naive_utc() - Duration::days(1);

    let (app, _anon, user) = TestApp::init().with_user();
    lock_account(&app, user.as_model().id, Some(until));

    user.async_get::<serde_json::Value>(URL).await.good();
}
