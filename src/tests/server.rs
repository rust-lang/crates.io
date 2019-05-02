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
