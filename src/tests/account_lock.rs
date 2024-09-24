use crate::{util::RequestHelper, TestApp};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use http::StatusCode;
use insta::assert_snapshot;

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

    let response = user.get::<()>(URL).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"This account is indefinitely locked. Reason: test lock reason"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn account_locked_with_future_expiry() {
    let until = "2099-12-12T12:12:12Z"
        .parse::<DateTime<Utc>>()
        .unwrap()
        .naive_utc();

    let (app, _anon, user) = TestApp::init().with_user();
    lock_account(&app, user.as_model().id, Some(until));

    let response = user.get::<()>(URL).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"This account is locked until 2099-12-12 at 12:12:12 UTC. Reason: test lock reason"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn expired_account_lock() {
    let until = Utc::now().naive_utc() - Duration::days(1);

    let (app, _anon, user) = TestApp::init().with_user();
    lock_account(&app, user.as_model().id, Some(until));

    user.get::<serde_json::Value>(URL).await.good();
}
