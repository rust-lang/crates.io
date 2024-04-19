use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use chrono::{NaiveDateTime, Utc};
use crates_io::rate_limiter::LimitedAction;
use crates_io::schema::{publish_limit_buckets, publish_rate_overrides};
use diesel::{ExpressionMethods, RunQueryDsl};
use http::StatusCode;
use std::thread;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn publish_new_crate_ratelimit_hit() {
    let (app, anon, _, token) = TestApp::full()
        .with_rate_limit(LimitedAction::PublishNew, Duration::from_millis(500), 1)
        .with_token();

    // Set up the database so it'll think we've massively ratelimited ourselves
    app.db(|conn| {
        // Ratelimit bucket should next refill in about a year
        let far_future = Utc::now().naive_utc() + Duration::from_secs(60 * 60 * 24 * 365);
        diesel::insert_into(publish_limit_buckets::table)
            .values((
                publish_limit_buckets::user_id.eq(token.as_model().user_id),
                publish_limit_buckets::action.eq(LimitedAction::PublishNew),
                publish_limit_buckets::tokens.eq(0),
                publish_limit_buckets::last_refill.eq(far_future),
            ))
            .execute(conn)
            .expect("Failed to set fake ratelimit")
    });

    let crate_to_publish = PublishBuilder::new("rate_limited", "1.0.0");
    token
        .publish_crate(crate_to_publish)
        .await
        .assert_rate_limited(LimitedAction::PublishNew);

    assert_eq!(app.stored_files().await.len(), 0);

    let response = anon.get::<()>("/api/v1/crates/rate_limited").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn publish_new_crate_ratelimit_expires() {
    let (app, anon, _, token) = TestApp::full()
        .with_rate_limit(LimitedAction::PublishNew, Duration::from_millis(500), 1)
        .with_token();

    // Set up the database so it'll think we've massively ratelimited ourselves
    app.db(|conn| {
        // Ratelimit bucket should next refill right now!
        let just_now = Utc::now().naive_utc() - Duration::from_millis(500);
        diesel::insert_into(publish_limit_buckets::table)
            .values((
                publish_limit_buckets::user_id.eq(token.as_model().user_id),
                publish_limit_buckets::action.eq(LimitedAction::PublishNew),
                publish_limit_buckets::tokens.eq(0),
                publish_limit_buckets::last_refill.eq(just_now),
            ))
            .execute(conn)
            .expect("Failed to set fake ratelimit")
    });

    let crate_to_publish = PublishBuilder::new("rate_limited", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();

    assert_eq!(app.stored_files().await.len(), 2);

    let json = anon.show_crate("rate_limited").await;
    assert_eq!(json.krate.max_version, "1.0.0");
}

#[tokio::test(flavor = "multi_thread")]
async fn publish_new_crate_override_loosens_ratelimit() {
    let (app, anon, _, token) = TestApp::full()
        // Most people get 1 new token every 1 day
        .with_rate_limit(
            LimitedAction::PublishNew,
            Duration::from_secs(60 * 60 * 24),
            1,
        )
        .with_token();

    app.db(|conn| {
        // Add an override so our user gets *2* new tokens (expires, y'know, sometime)
        diesel::insert_into(publish_rate_overrides::table)
            .values((
                publish_rate_overrides::user_id.eq(token.as_model().user_id),
                publish_rate_overrides::burst.eq(2),
                publish_rate_overrides::expires_at.eq(None::<NaiveDateTime>),
                publish_rate_overrides::action.eq(LimitedAction::PublishNew),
            ))
            .execute(conn)
            .expect("Failed to add ratelimit override")
    });

    let crate_to_publish = PublishBuilder::new("rate_limited1", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();

    assert_eq!(app.stored_files().await.len(), 2);

    let json = anon.show_crate("rate_limited1").await;
    assert_eq!(json.krate.max_version, "1.0.0");

    let crate_to_publish = PublishBuilder::new("rate_limited2", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();

    assert_eq!(app.stored_files().await.len(), 4);

    let json = anon.show_crate("rate_limited2").await;
    assert_eq!(json.krate.max_version, "1.0.0");

    let crate_to_publish = PublishBuilder::new("rate_limited3", "1.0.0");
    token
        .publish_crate(crate_to_publish)
        .await
        .assert_rate_limited(LimitedAction::PublishNew);

    assert_eq!(app.stored_files().await.len(), 4);

    let response = anon.get::<()>("/api/v1/crates/rate_limited3").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn publish_new_crate_expired_override_ignored() {
    let (app, anon, _, token) = TestApp::full()
        // Most people get 1 new token every 1 day
        .with_rate_limit(
            LimitedAction::PublishNew,
            Duration::from_secs(60 * 60 * 24),
            1,
        )
        .with_token();

    app.db(|conn| {
        // Add an override so our user gets *2* new tokens (expires, y'know, sometime)
        let just_now = Utc::now().naive_utc() - Duration::from_secs(1);
        diesel::insert_into(publish_rate_overrides::table)
            .values((
                publish_rate_overrides::user_id.eq(token.as_model().user_id),
                publish_rate_overrides::burst.eq(2),
                publish_rate_overrides::expires_at.eq(just_now),
                publish_rate_overrides::action.eq(LimitedAction::PublishNew),
            ))
            .execute(conn)
            .expect("Failed to add ratelimit override")
    });

    let crate_to_publish = PublishBuilder::new("rate_limited1", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();

    assert_eq!(app.stored_files().await.len(), 2);

    let json = anon.show_crate("rate_limited1").await;
    assert_eq!(json.krate.max_version, "1.0.0");

    let crate_to_publish = PublishBuilder::new("rate_limited2", "1.0.0");
    token
        .publish_crate(crate_to_publish)
        .await
        .assert_rate_limited(LimitedAction::PublishNew);

    assert_eq!(app.stored_files().await.len(), 2);

    let response = anon.get::<()>("/api/v1/crates/rate_limited2").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn publish_new_crate_rate_limit_doesnt_affect_existing_crates() {
    let (_, _, _, token) = TestApp::full()
        .with_rate_limit(LimitedAction::PublishNew, Duration::from_secs(60 * 60), 1)
        .with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("rate_limited1", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();

    let new_version = PublishBuilder::new("rate_limited1", "1.0.1");
    token.publish_crate(new_version).await.good();
}

#[tokio::test(flavor = "multi_thread")]
async fn publish_existing_crate_rate_limited() {
    let (app, anon, _, token) = TestApp::full()
        .with_rate_limit(LimitedAction::PublishUpdate, Duration::from_millis(500), 1)
        .with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("rate_limited1", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();

    let json = anon.show_crate("rate_limited1").await;
    assert_eq!(json.krate.max_version, "1.0.0");
    assert_eq!(app.stored_files().await.len(), 2);

    // Uploading the first update to the crate works
    let crate_to_publish = PublishBuilder::new("rate_limited1", "1.0.1");
    token.publish_crate(crate_to_publish).await.good();

    let json = anon.show_crate("rate_limited1").await;
    assert_eq!(json.krate.max_version, "1.0.1");
    assert_eq!(app.stored_files().await.len(), 3);

    // Uploading the second update to the crate is rate limited
    let crate_to_publish = PublishBuilder::new("rate_limited1", "1.0.2");
    token
        .publish_crate(crate_to_publish)
        .await
        .assert_rate_limited(LimitedAction::PublishUpdate);

    // Check that  version 1.0.2 was not published
    let json = anon.show_crate("rate_limited1").await;
    assert_eq!(json.krate.max_version, "1.0.1");
    assert_eq!(app.stored_files().await.len(), 3);

    // Wait for the limit to be up
    thread::sleep(Duration::from_millis(500));

    let crate_to_publish = PublishBuilder::new("rate_limited1", "1.0.2");
    token.publish_crate(crate_to_publish).await.good();

    let json = anon.show_crate("rate_limited1").await;
    assert_eq!(json.krate.max_version, "1.0.2");
    assert_eq!(app.stored_files().await.len(), 4);
}

#[tokio::test(flavor = "multi_thread")]
async fn publish_existing_crate_rate_limit_doesnt_affect_new_crates() {
    let (_, _, _, token) = TestApp::full()
        .with_rate_limit(
            LimitedAction::PublishUpdate,
            Duration::from_secs(60 * 60),
            1,
        )
        .with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("rate_limited1", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();

    // Upload a second new crate
    let crate_to_publish = PublishBuilder::new("rate_limited2", "1.0.0");
    token.publish_crate(crate_to_publish).await.good();
}
