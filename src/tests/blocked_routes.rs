use crate::builders::{CrateBuilder, VersionBuilder};
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;

#[test]
fn test_non_blocked_download_route() {
    let (app, anon, user) = TestApp::init()
        .with_config(|config| {
            config.blocked_routes.clear();
        })
        .with_user();

    app.db(|conn| {
        CrateBuilder::new("foo", user.as_model().id)
            .version(VersionBuilder::new("1.0.0"))
            .expect_build(conn);
    });

    let status = anon.get::<()>("/api/v1/crates/foo/1.0.0/download").status();
    assert_eq!(status, StatusCode::FOUND);
}

#[test]
fn test_blocked_download_route() {
    let (app, anon, user) = TestApp::init()
        .with_config(|config| {
            config.blocked_routes.clear();
            config
                .blocked_routes
                .insert("/api/v1/crates/:crate_id/:version/download".into());
        })
        .with_user();

    app.db(|conn| {
        CrateBuilder::new("foo", user.as_model().id)
            .version(VersionBuilder::new("1.0.0"))
            .expect_build(conn);
    });

    let status = anon.get::<()>("/api/v1/crates/foo/1.0.0/download").status();
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}
