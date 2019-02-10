// Note: Temporarily disabled because cargo-vendor doesn't include a
// User-Agent header and Rust's CI broke. If this is still commented out
// by Nov 7, 2018 ping the crates.io team.

// use conduit::{Handler, Method};
//
// use {app, req};
//
//
// #[test]
// fn user_agent_is_required() {
//     let (_b, _app, middle) = app();
//
//     let mut req = req(Method::Get, "/api/v1/crates");
//     req.header("User-Agent", "");
//     let resp = t!(middle.call(&mut req));
//     assert_eq!(resp.status.0, 403);
// }
