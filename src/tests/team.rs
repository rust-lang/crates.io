use std::sync::ONCE_INIT;
use conduit::{Handler, Method};

use cargo_registry::User;
use record::GhUser;

// Users: `crates-tester-1` and `crates-tester-2`
// Passwords: ask acrichto or gankro
// Teams: `crates-test-org:owners`, `crates-test-org:just-for-crates-2`
// tester-1 is on owners only, tester-2 is on both

static GH_USER_1: GhUser = GhUser { login: "crates-tester-1", init: ONCE_INIT };
static GH_USER_2: GhUser = GhUser { login: "crates-tester-2", init: ONCE_INIT };

fn mock_user_on_only_x() -> User { GH_USER_1.user() }
fn mock_user_on_x_and_y() -> User { GH_USER_2.user() }

fn body_for_team_y() -> &'static str {
    r#"{"users":["github:crates-test-org:just-for-crates-2"]}"#
}

fn body_for_team_x() -> &'static str {
    r#"{"users":["github:crates-test-org:owners"]}"#
}

// Test adding team without `github:`
#[test]
fn not_github() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo_not_github"));

    let body = r#"{"users":["dropbox:foo:foo"]}"#;
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/foo_not_github/owners")
                                        .with_method(Method::Put)
                                        .with_body(body.as_bytes())));
    assert!(json.errors[0].detail.contains("unknown organization"),
            "{:?}", json.errors);
}

#[test]
fn weird_name() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo_weird_name"));

    let body = r#"{"users":["github:foo/../bar:wut"]}"#;
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/foo_weird_name/owners")
                                        .with_method(Method::Put)
                                        .with_body(body.as_bytes())));
    assert!(json.errors[0].detail.contains("organization cannot contain"),
            "{:?}", json.errors);
}

// Test adding team without second `:`
#[test]
fn one_colon() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo_one_colon"));

    let body = r#"{"users":["github:foo"]}"#;
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/foo_one_colon/owners")
                                        .with_method(Method::Put)
                                        .with_body(body.as_bytes())));
    assert!(json.errors[0].detail.contains("missing github team"),
            "{:?}", json.errors);
}

#[test]
fn nonexistent_team() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo_nonexistent"));

    let body = r#"{"users":["github:crates-test-org:this-does-not-exist"]}"#;
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/foo_nonexistent/owners")
                                        .with_method(Method::Put)
                                        .with_body(body.as_bytes())));
    assert!(json.errors[0].detail.contains("could not find the github team"),
            "{:?}", json.errors);
}

// Test adding team as owner when on it
#[test]
fn add_team_as_member() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo_team_member"));

    let body = body_for_team_x();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo_team_member/owners")
                            .with_method(Method::Put)
                            .with_body(body.as_bytes())));
}

// Test adding team as owner when not on in
#[test]
fn add_team_as_non_member() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_only_x());
    ::mock_crate(&mut req, ::krate("foo_team_non_member"));

    let body = body_for_team_y();
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/foo_team_non_member/owners")
                                        .with_method(Method::Put)
                                        .with_body(body.as_bytes())));
    assert!(json.errors[0].detail.contains("only members"),
            "{:?}", json.errors);
}

#[test]
#[ignore] // FIXME: This test needs the owners endpoints hitting Diesel
fn remove_team_as_named_owner() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo_remove_team", "1.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo_remove_team"));

    let body = body_for_team_x();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo_remove_team/owners")
                            .with_method(Method::Put)
                            .with_body(body.as_bytes())));

    let body = body_for_team_x();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo_remove_team/owners")
                            .with_method(Method::Delete)
                            .with_body(body.as_bytes())));

    ::mock_user(&mut req, mock_user_on_only_x());
    let body = ::new_req_body_version_2(::krate("foo_remove_team"));
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/new")
                                        .with_body(&body)
                                        .with_method(Method::Put)));
    assert!(json.errors[0].detail.contains("another user"),
            "{:?}", json.errors);
}

#[test]
#[ignore] // FIXME: This test needs the owners endpoints hitting Diesel
fn remove_team_as_team_owner() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo_remove_team_owner", "1.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo_remove_team_owner"));

    let body = body_for_team_x();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo_remove_team_owner/owners")
                            .with_method(Method::Put)
                            .with_body(body.as_bytes())));

    ::mock_user(&mut req, mock_user_on_only_x());
    let body = body_for_team_x();
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/foo_remove_team_owner/owners")
                                        .with_method(Method::Delete)
                                        .with_body(body.as_bytes())));

    assert!(json.errors[0].detail.contains("don't have permission"),
            "{:?}", json.errors);

    let body = ::new_req_body_version_2(::krate("foo_remove_team_owner"));
    ok_resp!(middle.call(req.with_path("/api/v1/crates/new")
                            .with_body(&body)
                            .with_method(Method::Put)));
}

// Test trying to publish a krate we don't own
#[test]
#[ignore] // FIXME: This test needs the owners endpoints hitting Diesel
fn publish_not_owned() {
    let (_b, app, middle) = ::app();

    let mut req = ::new_req(app.clone(), "foo_not_owned", "1.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo_not_owned"));

    let body = body_for_team_y();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo_not_owned/owners")
                            .with_method(Method::Put)
                            .with_body(body.as_bytes())));

    ::mock_user(&mut req, mock_user_on_only_x());
    let body = ::new_req_body_version_2(::krate("foo_not_owned"));
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/new")
                                        .with_body(&body)
                                        .with_method(Method::Put)));
    assert!(json.errors[0].detail.contains("another user"),
            "{:?}", json.errors);
}

// Test trying to publish a krate we do own (but only because of teams)
#[test]
#[ignore] // FIXME: This test needs the owners endpoints hitting Diesel
fn publish_owned() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foo_team_owned", "1.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo_team_owned"));

    let body = body_for_team_x();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo_team_owned/owners")
                            .with_method(Method::Put)
                            .with_body(body.as_bytes())));

    ::mock_user(&mut req, mock_user_on_only_x());
    let body = ::new_req_body_version_2(::krate("foo_team_owned"));
    ok_resp!(middle.call(req.with_path("/api/v1/crates/new")
                            .with_body(&body)
                            .with_method(Method::Put)));
}

// Test trying to change owners (when only on an owning team)
#[test]
fn add_owners_as_team_owner() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foo_add_owner", "1.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo_add_owner"));

    let body = body_for_team_x();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo_add_owner/owners")
                            .with_method(Method::Put)
                            .with_body(body.as_bytes())));

    ::mock_user(&mut req, mock_user_on_only_x());
    let body = r#"{"users":["FlashCat"]}"#;     // User doesn't matter
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/foo_add_owner/owners")
                                        .with_method(Method::Put)
                                        .with_body(body.as_bytes())));
    assert!(json.errors[0].detail.contains("don't have permission"),
            "{:?}", json.errors);
}

