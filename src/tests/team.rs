use conduit::{Handler, Method};

use cargo_registry::User;

// Users: `crates-tester-1` and `crates-tester-2`
// Passwords: crates.io4lyfe
// Teams: `crates-test-org:owners`, `crates-test-org:just-for-crates-2`
// tester-1 is on owners only, tester-2 is on both

fn mock_user_on_x_and_y() -> User {
    User {
        id: 10000,
        gh_login: "crates-tester-2".to_string(),
        email: None,
        name: None,
        avatar: None,
        gh_access_token: "9d86670273ea0f7f51b9ed708d144267e0700b51".to_string(),
        api_token: User::new_api_token(),
    }
}

fn mock_user_on_only_x() -> User {
    User {
        id: 10000,
        gh_login: "crates-tester-1".to_string(),
        email: None,
        name: None,
        avatar: None,
        gh_access_token: "882faef00425a6b0e8f6750b7b7f7e295d5e42d3".to_string(),
        api_token: User::new_api_token(),
    }
}

fn body_for_add_team_y() -> &'static str {
    r#"{"users":["github:crates-test-org:just-for-crates-2"]}"#
}

fn body_for_add_team_x() -> &'static str {
    r#"{"users":["github:crates-test-org:owners"]}"#
}


// Test adding team without `github:`
#[test]
fn not_github() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo"));

    let body = r#"{"users":["dropbox:foo:foo"]}"#;
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/foo/owners")
                                       .with_method(Method::Put)
                                       .with_body(body.as_bytes())));
    assert!(json.errors[0].detail.contains("unknown organization"),
            "{:?}", json.errors);
}

// Test adding team without second `:`
#[test]
fn one_colon() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo"));

    let body = r#"{"users":["github:foo"]}"#;
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/foo/owners")
                                       .with_method(Method::Put)
                                       .with_body(body.as_bytes())));
    assert!(json.errors[0].detail.contains("missing github team"),
            "{:?}", json.errors);
}

// Test adding team as owner when on it
#[test]
fn add_team_as_member() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo"));

    let body = body_for_add_team_x();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo/owners")
                           .with_method(Method::Put)
                           .with_body(body.as_bytes())));
}

// Test adding team as owner when not on in
#[test]
fn add_team_as_non_member() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_only_x());
    ::mock_crate(&mut req, ::krate("foo"));

    let body = body_for_add_team_y();
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/foo/owners")
                           .with_method(Method::Put)
                           .with_body(body.as_bytes())));
    assert!(json.errors[0].detail.contains("only members"),
            "{:?}", json.errors);
}


// Test trying to publish a krate we don't own
#[test]
fn publish_not_owned() {
    let (_b, app, middle) = ::app();

    let mut req = ::new_req(app.clone(), "foo", "1.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo"));

    let body = body_for_add_team_y();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo/owners")
                           .with_method(Method::Put)
                           .with_body(body.as_bytes())));

    ::mock_user(&mut req, mock_user_on_only_x());
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/new")
                           .with_body(&::new_req_body(::krate("foo"),
                                        "2.0.0", vec![]))
                           .with_method(Method::Put)));
    assert!(json.errors[0].detail.contains("another user"),
            "{:?}", json.errors);
}

// Test trying to publish a krate we do own (but only because of teams)
#[test]
fn publish_owned() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foo", "1.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo"));

    let body = body_for_add_team_x();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo/owners")
                           .with_method(Method::Put)
                           .with_body(body.as_bytes())));

    ::mock_user(&mut req, mock_user_on_only_x());
    ok_resp!(middle.call(req.with_path("/api/v1/crates/new")
                           .with_body(&::new_req_body(::krate("foo"),
                                        "2.0.0", vec![]))
                           .with_method(Method::Put)));
}

// Test trying to change owners (when only on an owning team)
#[test]
fn change_owners() {
    let (_b, app, middle) = ::app();
    let mut req = ::new_req(app.clone(), "foo", "1.0.0");
    ::mock_user(&mut req, mock_user_on_x_and_y());
    ::mock_crate(&mut req, ::krate("foo"));

    let body = body_for_add_team_x();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo/owners")
                           .with_method(Method::Put)
                           .with_body(body.as_bytes())));

    ::mock_user(&mut req, mock_user_on_only_x());
    let body = r#"{"users":["FlashCat"]}"#;     // User doesn't matter
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/foo/owners")
                                       .with_method(Method::Put)
                                       .with_body(body.as_bytes())));
    assert!(json.errors[0].detail.contains("don't have permission"),
            "{:?}", json.errors);
}

