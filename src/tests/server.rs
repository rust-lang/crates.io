use conduit::Method;

use {app, req};

#[test]
fn user_agent_is_required() {
    let (_b, _app, middle) = app();

    let mut req = req(Method::Get, "/api/v1/crates");
    req.header("User-Agent", "");
    bad_resp!(middle.call(&mut req));
}
