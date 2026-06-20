use crate::builders::*;
use crate::util::*;
use std::collections::HashSet;

use ::insta::assert_json_snapshot;
use http::{Request, StatusCode, header};

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
    let mut conn = app.db_conn().await;

    CrateBuilder::new("dl_no_ua", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    let uri = "/api/v1/crates/dl_no_ua/0.99.0/download";
    let req = Request::get(uri).body("").unwrap();
    let resp = anon.run::<()>(req).await;
    assert_eq!(resp.status(), StatusCode::FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn blocked_traffic_doesnt_panic_if_checked_header_is_not_present() {
    let (app, anon, user) = TestApp::init()
        .with_config(|config| {
            config.block.traffic = vec![("Never-Given".into(), vec!["1".try_into().unwrap()])];
        })
        .with_user()
        .await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new("dl_no_ua", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    let uri = "/api/v1/crates/dl_no_ua/0.99.0/download";
    let req = Request::get(uri).body("").unwrap();
    let resp = anon.run::<()>(req).await;
    assert_eq!(resp.status(), StatusCode::FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn block_traffic_via_arbitrary_header_and_value() {
    let (app, anon, user) = TestApp::init()
        .with_config(|config| {
            config.block.traffic = vec![(
                "User-Agent".into(),
                vec![
                    // This is an exact string match because it doesn't start with `/`
                    "1/".try_into().unwrap(),
                    // This is also an exact string match, not interpreted as regex without slashes
                    "2+".try_into().unwrap(),
                    // Last two are regexes
                    "/fancy-crate, run by fancy-author v[\\d]+\\.[\\d]+\\.[\\d]+/"
                        .try_into()
                        .unwrap(),
                    "/^anchored$/".try_into().unwrap(),
                ],
            )];
        })
        .with_user()
        .await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new("dl_no_ua", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    let req = Request::get("/api/v1/crates/dl_no_ua/0.99.0/download")
        // A request with a header value we want to block isn't allowed
        .header(header::USER_AGENT, "2+")
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
            "1/value-must-match-exactly-this-is-allowed",
        )
        .body("")
        .unwrap();

    let resp = anon.run::<()>(req).await;
    assert_eq!(resp.status(), StatusCode::FOUND);

    let req = Request::get("/api/v1/crates/dl_no_ua/0.99.0/download")
        // A request with a header value we want to block via regex isn't allowed
        .header(
            header::USER_AGENT,
            "fancy-crate, run by fancy-author v14.105.6234",
        )
        .body("")
        .unwrap();

    let resp = anon.run::<()>(req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let req = Request::get("/api/v1/crates/dl_no_ua/0.99.0/download")
        // A request with a header value that has a partial match for the regex we want to block
        // isn't allowed because we didn't anchor the regex
        .header(
            header::USER_AGENT,
            "fancy-crate, run by fancy-author v1.2.3 oh and other stuff too",
        )
        .header("x-request-id", "abcd")
        .body("")
        .unwrap();
    let resp = anon.run::<()>(req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let req = Request::get("/api/v1/crates/dl_no_ua/0.99.0/download")
        // A request with a header value that exactly matches an anchored regex isn't allowed
        .header(header::USER_AGENT, "anchored")
        .body("")
        .unwrap();

    let resp = anon.run::<()>(req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let req = Request::get("/api/v1/crates/dl_no_ua/0.99.0/download")
        // A request with a header value that doesn't match an anchored regex is allowed
        .header(header::USER_AGENT, "anchored, it's a pirate's life for me")
        .body("")
        .unwrap();

    let resp = anon.run::<()>(req).await;
    assert_eq!(resp.status(), StatusCode::FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn block_traffic_via_ip() {
    let (_app, anon) = TestApp::init()
        .with_config(|config| {
            config.block.ips = HashSet::from(["127.0.0.1".parse().unwrap()]);
        })
        .empty()
        .await;

    let resp = anon.get::<()>("/api/v1/crates").await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_json_snapshot!(resp.json());
}
