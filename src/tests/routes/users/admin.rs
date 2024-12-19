use chrono::{DateTime, Utc};
use crates_io_database::schema::users;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};
use serde_json::json;

use crate::{
    models::User,
    tests::util::{RequestHelper, TestApp},
};

mod get {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn get() {
        let (app, anon, user) = TestApp::init().with_user().await;
        let admin = app.db_new_admin_user("admin").await;

        // Anonymous users should be forbidden.
        let response = anon.get::<()>("/api/v1/users/foo/admin").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("anonymous-found", response.text());

        let response = anon.get::<()>("/api/v1/users/bar/admin").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("anonymous-not-found", response.text());

        // Regular users should also be forbidden, even if they're requesting
        // themself.
        let response = user.get::<()>("/api/v1/users/foo/admin").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("non-admin-found", response.text());

        let response = user.get::<()>("/api/v1/users/bar/admin").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("non-admin-not-found", response.text());

        // Admin users are allowed, but still can't manifest users who don't
        // exist.
        let response = admin.get::<()>("/api/v1/users/bar/admin").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_snapshot!("admin-not-found", response.text());

        // Admin users are allowed, and should get an admin's eye view of the
        // requested user.
        let response = admin.get::<()>("/api/v1/users/foo/admin").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_json_snapshot!("admin-found", response.json());
    }
}

mod lock {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn lock() {
        let (app, anon, user) = TestApp::init().with_user().await;
        let admin = app.db_new_admin_user("admin").await;

        // Because axum will validate and deserialise the body before any auth
        // check occurs, we actually need to provide a valid body for all the
        // auth related test cases.
        let body = serde_json::to_string(&json!({
            "reason": "l33t h4x0r",
            "until": "2045-01-01T01:02:03Z",
        }))
        .unwrap();

        // Anonymous users should be forbidden.
        let response = anon.put::<()>("/api/v1/users/foo/lock", body.clone()).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("anonymous-found", response.text());

        let response = anon.put::<()>("/api/v1/users/bar/lock", body.clone()).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("anonymous-not-found", response.text());

        // Regular users should also be forbidden, even if they're locking
        // themself.
        let response = user.put::<()>("/api/v1/users/foo/lock", body.clone()).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("non-admin-found", response.text());

        let response = user.put::<()>("/api/v1/users/bar/lock", body.clone()).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("non-admin-not-found", response.text());

        // Admin users are allowed, but still can't manifest users who don't
        // exist.
        let response = admin
            .put::<()>("/api/v1/users/bar/lock", body.clone())
            .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_snapshot!("admin-not-found", response.text());

        // Admin users who provide invalid request bodies should be denied.
        let response = admin
            .put::<()>("/api/v1/users/bar/lock", b"invalid JSON" as &[u8])
            .await;
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_snapshot!("admin-invalid-json", response.text());

        let response = admin
            .put::<()>("/api/v1/users/bar/lock", br#"{"valid": "json"}"# as &[u8])
            .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_snapshot!("admin-malformed-json", response.text());

        // Admin users are allowed, and should be able to lock the user.
        assert_none!(&user.as_model().account_lock_reason);
        assert_none!(&user.as_model().account_lock_until);

        let response = admin.put::<()>("/api/v1/users/foo/lock", body).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_json_snapshot!("admin-found", response.json());

        // Get the user again and validate that they are now locked.
        let mut conn = app.db_conn().await;
        let locked_user = User::find(&mut conn, user.as_model().id).await.unwrap();
        assert_user_is_locked(&locked_user, "l33t h4x0r", "2045-01-01T01:02:03Z");

        // Re-locking a locked user should update their lock reason and
        // expiration time.
        let body = serde_json::to_string(&json!({
            "reason": "less l33t",
            "until": "2035-01-01T01:02:03Z",
        }))
        .unwrap();

        let response = admin.put::<()>("/api/v1/users/foo/lock", body).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_json_snapshot!("admin-relock-shorter", response.json());

        // Get the user again and validate that they are now locked for less
        // time.
        let mut conn = app.db_conn().await;
        let locked_user = User::find(&mut conn, user.as_model().id).await.unwrap();
        assert_user_is_locked(&locked_user, "less l33t", "2035-01-01T01:02:03Z");

        // Finally, not including an until time at all should lock the account
        // forever. (Insert evil laughter here.)
        let body = serde_json::to_string(&json!({
            "reason": "less l33t",
        }))
        .unwrap();

        let response = admin.put::<()>("/api/v1/users/foo/lock", body).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_json_snapshot!("admin-lock-forever", response.json());

        // Get the user again and validate that they are now locked forever.
        let mut conn = app.db_conn().await;
        let locked_user = User::find(&mut conn, user.as_model().id).await.unwrap();
        assert_user_is_locked_indefinitely(&locked_user, "less l33t");
    }
}

mod unlock {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn unlock() {
        let (app, anon, user) = TestApp::init().with_user().await;
        let admin = app.db_new_admin_user("admin").await;

        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;

        // First up, let's lock the user.
        let mut conn = app.db_conn().await;
        diesel::update(user.as_model())
            .set((
                users::account_lock_reason.eq("naughty naughty"),
                users::account_lock_until.eq(DateTime::parse_from_rfc3339("2050-01-01T01:02:03Z")
                    .unwrap()
                    .naive_utc()),
            ))
            .execute(&mut conn)
            .await
            .unwrap();

        // Anonymous users should be forbidden.
        let response = anon.delete::<()>("/api/v1/users/foo/lock").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("anonymous-found", response.text());

        let response = anon.delete::<()>("/api/v1/users/bar/lock").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("anonymous-not-found", response.text());

        // Regular users should also be forbidden, even if they're locking
        // themself.
        let response = user.delete::<()>("/api/v1/users/foo/lock").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("non-admin-found", response.text());

        let response = user.delete::<()>("/api/v1/users/bar/lock").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("non-admin-not-found", response.text());

        // Admin users are allowed, but still can't manifest users who don't
        // exist.
        let response = admin.delete::<()>("/api/v1/users/bar/lock").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_snapshot!("admin-not-found", response.text());

        // Admin users are allowed, and should be able to unlock the user.
        let response = admin.delete::<()>("/api/v1/users/foo/lock").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_json_snapshot!("admin-found", response.json(), {
            ".lock.until" => "[datetime]",
        });

        // Get the user again and validate that they are now unlocked.
        let mut conn = app.db_conn().await;
        let unlocked_user = User::find(&mut conn, user.as_model().id).await.unwrap();
        assert_user_is_unlocked(&unlocked_user);

        // Unlocking an unlocked user should succeed silently.
        let response = admin.delete::<()>("/api/v1/users/foo/lock").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_json_snapshot!("admin-reunlock", response.json(), {
            ".lock.until" => "[datetime]",
        });
    }
}

#[track_caller]
fn assert_user_is_locked(user: &User, reason: &str, until: &str) {
    assert_eq!(user.account_lock_reason.as_deref(), Some(reason));
    assert_eq!(
        user.account_lock_until,
        Some(DateTime::parse_from_rfc3339(until).unwrap().naive_utc())
    );
}

#[track_caller]
fn assert_user_is_locked_indefinitely(user: &User, reason: &str) {
    assert_eq!(user.account_lock_reason.as_deref(), Some(reason));
    assert_none!(user.account_lock_until);
}

#[track_caller]
fn assert_user_is_unlocked(user: &User) {
    if user.account_lock_reason.is_some() {
        if let Some(until) = user.account_lock_until {
            assert_lt!(until, Utc::now().naive_utc());
        } else {
            panic!("user account is locked indefinitely");
        }
    }
}
