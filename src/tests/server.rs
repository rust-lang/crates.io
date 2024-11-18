use crate::tests::builders::*;
use crate::tests::util::*;
use std::collections::HashSet;

use ::insta::assert_json_snapshot;
use http::{header, Request, StatusCode};

#[tokio::test(flavor = "multi_thread")]
async fn user_agent_is_required() {
    let (_app, anon) = TestApp::init().empty().await;

    let req = Request::get("/api/v1/crates").body("").unwrap();
    let resp = anon.run::<()>(req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_json_snapshot!(resp.json());

    let req = Request::get("/api/v1/crates")
        .header(header::USER_AGENT, "")
        .body("")
        .unwrap();
    let resp = anon.run::<()>(req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_json_snapshot!(resp.json());
}

#[tokio::test(flavor = "multi_thread")]
async fn user_agent_is_not_required_for_download() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn();

    CrateBuilder::new("dl_no_ua", user.as_model().id).expect_build(&mut conn);

    let uri = "/api/v1/crates/dl_no_ua/0.99.0/download";
    let req = Request::get(uri).body("").unwrap();
    let resp = anon.run::<()>(req).await;
    assert_eq!(resp.status(), StatusCode::FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn blocked_traffic_doesnt_panic_if_checked_header_is_not_present() {
    let (app, anon, user) = TestApp::init()
        .with_config(|config| {
            config.blocked_traffic = vec![("Never-Given".into(), vec!["1".into()])];
        })
        .with_user()
        .await;

    let mut conn = app.db_conn();

    CrateBuilder::new("dl_no_ua", user.as_model().id).expect_build(&mut conn);

    let uri = "/api/v1/crates/dl_no_ua/0.99.0/download";
    let req = Request::get(uri).body("").unwrap();
    let resp = anon.run::<()>(req).await;
    assert_eq!(resp.status(), StatusCode::FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn block_traffic_via_arbitrary_header_and_value() {
    let (app, anon, user) = TestApp::init()
        .with_config(|config| {
            config.blocked_traffic = vec![("User-Agent".into(), vec!["1".into(), "2".into()])];
        })
        .with_user()
        .await;

    let mut conn = app.db_conn();

    CrateBuilder::new("dl_no_ua", user.as_model().id).expect_build(&mut conn);

    let req = Request::get("/api/v1/crates/dl_no_ua/0.99.0/download")
        // A request with a header value we want to block isn't allowed
        .header(header::USER_AGENT, "1")
        .header("x-request-id", "abcd")
        .body("")
        .unwrap();

    let resp = anon.run::<()>(req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_json_snapshot!(resp.json());

    let req = Request::get("/api/v1/crates/dl_no_ua/0.99.0/download")
        // A request with a header value we don't want to block is allowed, even though there might
        // be a substring match
        .header(
            header::USER_AGENT,
            "1value-must-match-exactly-this-is-allowed",
        )
        .body("")
        .unwrap();

    let resp = anon.run::<()>(req).await;
    assert_eq!(resp.status(), StatusCode::FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn block_traffic_via_ip() {
    let (_app, anon) = TestApp::init()
        .with_config(|config| {
            config.blocked_ips = HashSet::from(["127.0.0.1".parse().unwrap()]);
        })
        .empty()
        .await;

    let resp = anon.get::<()>("/api/v1/crates").await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_json_snapshot!(resp.json());
}
