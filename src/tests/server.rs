use crate::builders::*;
use crate::util::*;

use conduit::{header, Method, StatusCode};

#[test]
fn user_agent_is_required() {
    let (_app, anon) = TestApp::init().empty();

    let mut req = anon.request_builder(Method::GET, "/api/v1/crates");
    req.header(header::USER_AGENT, "");
    let resp = anon.run::<()>(req);
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[test]
fn user_agent_is_not_required_for_download() {
    let (app, anon, user) = TestApp::init().with_user();

    app.db(|conn| {
        CrateBuilder::new("dl_no_ua", user.as_model().id).expect_build(conn);
    });

    let mut req = anon.request_builder(Method::GET, "/api/v1/crates/dl_no_ua/0.99.0/download");
    req.header(header::USER_AGENT, "");
    let resp = anon.run::<()>(req);
    assert_eq!(resp.status(), StatusCode::FOUND);
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

    let mut req = anon.request_builder(Method::GET, "/api/v1/crates/dl_no_ua/0.99.0/download");
    req.header(header::USER_AGENT, "");
    let resp = anon.run::<()>(req);
    assert_eq!(resp.status(), StatusCode::FOUND);
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

    let mut req = anon.request_builder(Method::GET, "/api/v1/crates/dl_no_ua/0.99.0/download");
    // A request with a header value we want to block isn't allowed
    req.header(header::USER_AGENT, "1");
    req.header("x-request-id", "abcd");
    let resp = anon.run::<()>(req);
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let mut req = anon.request_builder(Method::GET, "/api/v1/crates/dl_no_ua/0.99.0/download");
    // A request with a header value we don't want to block is allowed, even though there might
    // be a substring match
    req.header(
        header::USER_AGENT,
        "1value-must-match-exactly-this-is-allowed",
    );
    let resp = anon.run::<()>(req);
    assert_eq!(resp.status(), StatusCode::FOUND);
}
