use crate::util::{RequestHelper, TestApp};
use deadpool_diesel::postgres::Pool;
use deadpool_diesel::Timeouts;
use http::StatusCode;
use std::time::Duration;

const DB_HEALTHY_TIMEOUT: Duration = Duration::from_millis(2000);

fn default_timeouts() -> Timeouts {
    Timeouts::wait_millis(DB_HEALTHY_TIMEOUT.as_millis() as u64)
}

fn wait_until_healthy(pool: &Pool, app: &TestApp) {
    let _ = app
        .runtime()
        .block_on(pool.timeout_get(&default_timeouts()))
        .expect("the database did not return healthy");
}

#[test]
fn http_error_with_unhealthy_database() {
    let (app, anon) = TestApp::init().with_chaos_proxy().empty();

    let response = anon.get::<()>("/api/v1/summary");
    assert_eq!(response.status(), StatusCode::OK);

    app.primary_db_chaosproxy().break_networking().unwrap();

    let response = anon.get::<()>("/api/v1/summary");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    app.primary_db_chaosproxy().restore_networking().unwrap();
    wait_until_healthy(&app.as_inner().deadpool_primary, &app);

    let response = anon.get::<()>("/api/v1/summary");
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn fallback_to_replica_returns_user_info() {
    const URL: &str = "/api/v1/users/foo";

    let (app, _, owner) = TestApp::init()
        .with_replica()
        .with_chaos_proxy()
        .with_user();
    app.db_new_user("foo");
    app.primary_db_chaosproxy().break_networking().unwrap();

    // When the primary database is down, requests are forwarded to the replica database
    let response = owner.get::<()>(URL);
    assert_eq!(response.status(), 200);

    // restore primary database connection
    app.primary_db_chaosproxy().restore_networking().unwrap();
    wait_until_healthy(&app.as_inner().deadpool_primary, &app);
}

#[test]
fn restored_replica_returns_user_info() {
    const URL: &str = "/api/v1/users/foo";

    let (app, _, owner) = TestApp::init()
        .with_replica()
        .with_chaos_proxy()
        .with_user();
    app.db_new_user("foo");
    app.primary_db_chaosproxy().break_networking().unwrap();
    app.replica_db_chaosproxy().break_networking().unwrap();

    // When both primary and replica database are down, the request returns an error
    let response = owner.get::<()>(URL);
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Once the replica database is restored, it should serve as a fallback again
    app.replica_db_chaosproxy().restore_networking().unwrap();
    let replica = app
        .as_inner()
        .deadpool_replica
        .as_ref()
        .expect("no replica database configured");
    wait_until_healthy(replica, &app);

    let response = owner.get::<()>(URL);
    assert_eq!(response.status(), StatusCode::OK);

    // restore connection
    app.primary_db_chaosproxy().restore_networking().unwrap();
    wait_until_healthy(&app.as_inner().deadpool_primary, &app);
}

#[test]
fn restored_primary_returns_user_info() {
    const URL: &str = "/api/v1/users/foo";

    let (app, _, owner) = TestApp::init()
        .with_replica()
        .with_chaos_proxy()
        .with_user();
    app.db_new_user("foo");
    app.primary_db_chaosproxy().break_networking().unwrap();
    app.replica_db_chaosproxy().break_networking().unwrap();

    // When both primary and replica database are down, the request returns an error
    let response = owner.get::<()>(URL);
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Once the replica database is restored, it should serve as a fallback again
    app.primary_db_chaosproxy().restore_networking().unwrap();
    wait_until_healthy(&app.as_inner().deadpool_primary, &app);

    let response = owner.get::<()>(URL);
    assert_eq!(response.status(), StatusCode::OK);
}
