use crate::builders::PublishBuilder;
use crate::routes::crates::versions::yank_unyank::YankRequestHelper;
use crate::util::{RequestHelper, TestApp};
use chrono::Utc;
use crates_io::rate_limiter::LimitedAction;
use crates_io::schema::publish_limit_buckets;
use diesel::{ExpressionMethods, RunQueryDsl};
use googletest::prelude::*;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
#[allow(unknown_lints, clippy::bool_assert_comparison)] // for claim::assert_some_eq! with bool
async fn yank_works_as_intended() {
    let (app, anon, cookie, token) = TestApp::full().with_token();

    // Upload a new crate, putting it in the git index
    let crate_to_publish = PublishBuilder::new("fyk", "1.0.0");
    token.async_publish_crate(crate_to_publish).await.good();

    let crates = app.crates_from_index_head("fyk");
    assert_that!(crates, len(eq(1)));
    assert_some_eq!(crates[0].yanked, false);

    // make sure it's not yanked
    let json = anon.async_show_version("fyk", "1.0.0").await;
    assert!(!json.version.yanked);

    // yank it
    token.async_yank("fyk", "1.0.0").await.good();

    let crates = app.crates_from_index_head("fyk");
    assert_that!(crates, len(eq(1)));
    assert_some_eq!(crates[0].yanked, true);

    let json = anon.async_show_version("fyk", "1.0.0").await;
    assert!(json.version.yanked);

    // un-yank it
    token.async_unyank("fyk", "1.0.0").await.good();

    let crates = app.crates_from_index_head("fyk");
    assert_that!(crates, len(eq(1)));
    assert_some_eq!(crates[0].yanked, false);

    let json = anon.async_show_version("fyk", "1.0.0").await;
    assert!(!json.version.yanked);

    // yank it
    cookie.async_yank("fyk", "1.0.0").await.good();

    let crates = app.crates_from_index_head("fyk");
    assert_that!(crates, len(eq(1)));
    assert_some_eq!(crates[0].yanked, true);

    let json = anon.async_show_version("fyk", "1.0.0").await;
    assert!(json.version.yanked);

    // un-yank it
    cookie.async_unyank("fyk", "1.0.0").await.good();

    let crates = app.crates_from_index_head("fyk");
    assert_that!(crates, len(eq(1)));
    assert_some_eq!(crates[0].yanked, false);

    let json = anon.async_show_version("fyk", "1.0.0").await;
    assert!(!json.version.yanked);
}

#[track_caller]
fn check_yanked(app: &TestApp, is_yanked: bool) {
    let crates = app.crates_from_index_head("yankable");
    assert_that!(crates, len(eq(1)));
    assert_some_eq!(crates[0].yanked, is_yanked);
}

#[tokio::test(flavor = "multi_thread")]
async fn yank_ratelimit_hit() {
    let (app, _, _, token) = TestApp::full()
        .with_rate_limit(LimitedAction::YankUnyank, Duration::from_millis(500), 1)
        .with_token();

    // Set up the database so it'll think we've massively rate-limited ourselves.
    app.db(|conn| {
        // Ratelimit bucket should next refill in about a year
        let far_future = Utc::now().naive_utc() + Duration::from_secs(60 * 60 * 24 * 365);
        diesel::insert_into(publish_limit_buckets::table)
            .values((
                publish_limit_buckets::user_id.eq(token.as_model().user_id),
                publish_limit_buckets::action.eq(LimitedAction::YankUnyank),
                publish_limit_buckets::tokens.eq(0),
                publish_limit_buckets::last_refill.eq(far_future),
            ))
            .execute(conn)
            .expect("Failed to set fake ratelimit")
    });

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("yankable", "1.0.0");
    token.async_publish_crate(crate_to_publish).await.good();
    check_yanked(&app, false);

    // Yank it and wait for the ratelimit to hit.
    token
        .async_yank("yankable", "1.0.0")
        .await
        .assert_rate_limited(LimitedAction::YankUnyank);
    check_yanked(&app, false);
}

#[tokio::test(flavor = "multi_thread")]
async fn yank_ratelimit_expires() {
    let (app, _, _, token) = TestApp::full()
        .with_rate_limit(LimitedAction::YankUnyank, Duration::from_millis(500), 1)
        .with_token();

    // Set up the database so it'll think we've massively ratelimited ourselves
    app.db(|conn| {
        // Ratelimit bucket should next refill right now!
        let just_now = Utc::now().naive_utc() - Duration::from_millis(500);
        diesel::insert_into(publish_limit_buckets::table)
            .values((
                publish_limit_buckets::user_id.eq(token.as_model().user_id),
                publish_limit_buckets::action.eq(LimitedAction::YankUnyank),
                publish_limit_buckets::tokens.eq(0),
                publish_limit_buckets::last_refill.eq(just_now),
            ))
            .execute(conn)
            .expect("Failed to set fake ratelimit")
    });

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("yankable", "1.0.0");
    token.async_publish_crate(crate_to_publish).await.good();
    check_yanked(&app, false);

    token.async_yank("yankable", "1.0.0").await.good();
    check_yanked(&app, true);
}

#[tokio::test(flavor = "multi_thread")]
async fn yank_max_version() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("fyk_max", "1.0.0");
    token.async_publish_crate(crate_to_publish).await.good();

    // double check the max version
    let json = anon.async_show_crate("fyk_max").await;
    assert_eq!(json.krate.max_version, "1.0.0");

    // add version 2.0.0
    let crate_to_publish = PublishBuilder::new("fyk_max", "2.0.0");
    let json = token.async_publish_crate(crate_to_publish).await.good();
    assert_eq!(json.krate.max_version, "2.0.0");

    // yank version 1.0.0
    token.async_yank("fyk_max", "1.0.0").await.good();

    let json = anon.async_show_crate("fyk_max").await;
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    token.async_unyank("fyk_max", "1.0.0").await.good();

    let json = anon.async_show_crate("fyk_max").await;
    assert_eq!(json.krate.max_version, "2.0.0");

    // yank version 2.0.0
    token.async_yank("fyk_max", "2.0.0").await.good();

    let json = anon.async_show_crate("fyk_max").await;
    assert_eq!(json.krate.max_version, "1.0.0");

    // yank version 1.0.0
    token.async_yank("fyk_max", "1.0.0").await.good();

    let json = anon.async_show_crate("fyk_max").await;
    assert_eq!(json.krate.max_version, "0.0.0");

    // unyank version 2.0.0
    token.async_unyank("fyk_max", "2.0.0").await.good();

    let json = anon.async_show_crate("fyk_max").await;
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    token.async_unyank("fyk_max", "1.0.0").await.good();

    let json = anon.async_show_crate("fyk_max").await;
    assert_eq!(json.krate.max_version, "2.0.0");
}

#[tokio::test(flavor = "multi_thread")]
async fn publish_after_yank_max_version() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("fyk_max", "1.0.0");
    token.async_publish_crate(crate_to_publish).await.good();

    // double check the max version
    let json = anon.async_show_crate("fyk_max").await;
    assert_eq!(json.krate.max_version, "1.0.0");

    // yank version 1.0.0
    token.async_yank("fyk_max", "1.0.0").await.good();

    let json = anon.async_show_crate("fyk_max").await;
    assert_eq!(json.krate.max_version, "0.0.0");

    // add version 2.0.0
    let crate_to_publish = PublishBuilder::new("fyk_max", "2.0.0");
    let json = token.async_publish_crate(crate_to_publish).await.good();
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    token.async_unyank("fyk_max", "1.0.0").await.good();

    let json = anon.async_show_crate("fyk_max").await;
    assert_eq!(json.krate.max_version, "2.0.0");
}
