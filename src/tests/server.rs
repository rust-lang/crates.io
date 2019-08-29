use conduit::Method;

use crate::builders::*;
use crate::util::*;

#[test]
fn user_agent_is_required() {
    let (_app, anon) = TestApp::init().empty();

    let mut req = anon.request_builder(Method::Get, "/api/v1/crates");
    req.header("User-Agent", "");
    let resp = anon.run::<()>(req);
    resp.assert_status(403);
}

#[test]
fn user_agent_is_not_required_for_download() {
    let (app, anon, user) = TestApp::init().with_user();

    app.db(|conn| {
        CrateBuilder::new("dl_no_ua", user.as_model().id).expect_build(conn);
    });

    let mut req = anon.request_builder(Method::Get, "/api/v1/crates/dl_no_ua/0.99.0/download");
    req.header("User-Agent", "");
    let resp = anon.run::<()>(req);
    resp.assert_status(302);
}

#[test]
fn blocked_traffic_doesnt_panic_if_checked_header_is_not_present() {
    let (app, anon, user) = TestApp::init()
        .with_config(|config| {
            config.blocked_traffic = vec![("Never-Given".into(), vec!["1".into()])];
        })
        .with_user();

    app.db(|conn| {
        CrateBuilder::new("dl_no_ua", user.as_model().id).expect_build(conn);
    });

    let mut req = anon.request_builder(Method::Get, "/api/v1/crates/dl_no_ua/0.99.0/download");
    req.header("User-Agent", "");
    let resp = anon.run::<()>(req);
    resp.assert_status(302);
}

#[test]
fn block_traffic_via_arbitrary_header_and_value() {
    let (app, anon, user) = TestApp::init()
        .with_config(|config| {
            config.blocked_traffic = vec![("User-Agent".into(), vec!["1".into(), "2".into()])];
        })
        .with_user();

    app.db(|conn| {
        CrateBuilder::new("dl_no_ua", user.as_model().id).expect_build(conn);
    });

    let mut req = anon.request_builder(Method::Get, "/api/v1/crates/dl_no_ua/0.99.0/download");
    // A request with a header value we want to block isn't allowed
    req.header("User-Agent", "1");
    req.header("X-Request-Id", "abcd"); // Needed for the error message we generate
    let resp = anon.run::<()>(req);
    resp.assert_status(403);

    let mut req = anon.request_builder(Method::Get, "/api/v1/crates/dl_no_ua/0.99.0/download");
    // A request with a header value we don't want to block is allowed, even though there might
    // be a substring match
    req.header("User-Agent", "1value-must-match-exactly-this-is-allowed");
    let resp = anon.run::<()>(req);
    resp.assert_status(302);
}
