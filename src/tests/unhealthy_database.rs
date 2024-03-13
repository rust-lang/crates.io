use crate::util::{RequestHelper, TestApp};
use http::StatusCode;
use std::time::Duration;

const DB_HEALTHY_TIMEOUT: Duration = Duration::from_millis(2000);

#[test]
fn http_error_with_unhealthy_database() {
    let (app, anon) = TestApp::init().with_chaos_proxy().empty();

    let response = anon.get::<()>("/api/v1/summary");
    assert_eq!(response.status(), StatusCode::OK);

    app.primary_db_chaosproxy().break_networking().unwrap();

    let response = anon.get::<()>("/api/v1/summary");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    app.primary_db_chaosproxy().restore_networking().unwrap();
    app.as_inner()
        .primary_database
        .wait_until_healthy(DB_HEALTHY_TIMEOUT)
        .expect("the database did not return healthy");

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
    app.as_inner()
        .primary_database
        .wait_until_healthy(DB_HEALTHY_TIMEOUT)
        .expect("the database did not return healthy");
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
    app.as_inner()
        .read_only_replica_database
        .as_ref()
        .expect("no replica database configured")
        .wait_until_healthy(DB_HEALTHY_TIMEOUT)
        .expect("the database did not return healthy");

    let response = owner.get::<()>(URL);
    assert_eq!(response.status(), StatusCode::OK);

    // restore connection
    app.primary_db_chaosproxy().restore_networking().unwrap();
    app.as_inner()
        .primary_database
        .wait_until_healthy(DB_HEALTHY_TIMEOUT)
        .expect("the database did not return healthy");
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
    app.as_inner()
        .primary_database
        .wait_until_healthy(DB_HEALTHY_TIMEOUT)
        .expect("the database did not return healthy");

    let response = owner.get::<()>(URL);
    assert_eq!(response.status(), StatusCode::OK);
}
