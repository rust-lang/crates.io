use crate::{
    builders::CrateBuilder,
    util::{MockAnonymousUser, RequestHelper, TestApp},
};
use http::StatusCode;
use std::time::Duration;

#[test]
fn download_crate_with_broken_networking_primary_database() {
    let (app, anon, _, owner) = TestApp::init().with_slow_real_db_pool().with_token();
    app.db(|conn| {
        CrateBuilder::new("crate_name", owner.as_model().user_id)
            .version("1.0.0")
            .expect_build(conn)
    });

    // When the database connection is healthy downloads are redirected with the proper
    // capitalization, and missing crates or versions return a 404.

    assert_checked_redirects(&anon);

    // After networking breaks, preventing new database connections, the download endpoint should
    // do an unconditional redirect to the CDN, without checking whether the crate exists or what
    // the exact capitalization of crate name is.

    app.db_chaosproxy().break_networking();
    assert_unconditional_redirects(&anon);

    // After restoring the network and waiting for the database pool to get healthy again redirects
    // should be checked again.

    app.db_chaosproxy().restore_networking();
    app.as_inner()
        .primary_database
        .wait_until_healthy(Duration::from_millis(500))
        .expect("the database did not return healthy");

    assert_checked_redirects(&anon);
}

fn assert_checked_redirects(anon: &MockAnonymousUser) {
    anon.get::<()>("/api/v1/crates/crate_name/1.0.0/download")
        .assert_redirect_ends_with("/crate_name/crate_name-1.0.0.crate");

    anon.get::<()>("/api/v1/crates/Crate-Name/1.0.0/download")
        .assert_redirect_ends_with("/crate_name/crate_name-1.0.0.crate");

    anon.get::<()>("/api/v1/crates/crate_name/2.0.0/download")
        .assert_not_found();

    anon.get::<()>("/api/v1/crates/awesome-project/1.0.0/download")
        .assert_not_found();
}

fn assert_unconditional_redirects(anon: &MockAnonymousUser) {
    anon.get::<()>("/api/v1/crates/crate_name/1.0.0/download")
        .assert_redirect_ends_with("/crate_name/crate_name-1.0.0.crate");

    anon.get::<()>("/api/v1/crates/Crate-Name/1.0.0/download")
        .assert_redirect_ends_with("/Crate-Name/Crate-Name-1.0.0.crate");

    anon.get::<()>("/api/v1/crates/crate_name/2.0.0/download")
        .assert_redirect_ends_with("/crate_name/crate_name-2.0.0.crate");

    anon.get::<()>("/api/v1/crates/awesome-project/1.0.0/download")
        .assert_redirect_ends_with("/awesome-project/awesome-project-1.0.0.crate");
}

#[test]
fn http_error_with_unhealthy_database() {
    let (app, anon) = TestApp::init().with_slow_real_db_pool().empty();

    let response = anon.get::<()>("/api/v1/summary");
    assert_eq!(response.status(), StatusCode::OK);

    app.db_chaosproxy().break_networking();

    let response = anon.get::<()>("/api/v1/summary");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    app.db_chaosproxy().restore_networking();
    app.as_inner()
        .primary_database
        .wait_until_healthy(Duration::from_millis(500))
        .expect("the database did not return healthy");

    let response = anon.get::<()>("/api/v1/summary");
    assert_eq!(response.status(), StatusCode::OK);
}
