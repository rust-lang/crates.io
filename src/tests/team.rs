#![allow(unused_imports, dead_code)]

use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::{self, File};
use std::iter::repeat;
use std::sync::Arc;

use conduit::{Handler, Request, Method};
use conduit_test::MockRequest;
use conduit_middleware::MiddlewareBuilder;
use git2;
use rustc_serialize::{json, Decoder};
use semver;

use cargo_registry::App;
use cargo_registry::dependency::EncodableDependency;
use cargo_registry::download::EncodableVersionDownload;
use cargo_registry::krate::{Crate, EncodableCrate};
use cargo_registry::upload as u;
use cargo_registry::user::EncodableUser;
use cargo_registry::version::EncodableVersion;
use cargo_registry::User;

#[derive(RustcDecodable)]
struct AuthResponse { url: String, state: String }
#[derive(RustcDecodable)]
struct MeResponse { user: EncodableUser, api_token: String }

// Users: `crates-tester-1` and `crates-tester-2`
// Passwords: crates.io4lyfe
// Teams: `crates-test-org:owners`, `crates-test-org:just-for-crates-2`
// tester-1 is on owners only, tester-2 is on both

fn mock_user_on_both() -> User {
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

fn mock_user_on_one() -> User {
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

fn body_for_add_solo_team() -> &str {
    r#"{"users":["github:crates-test-org:just-for-crates-2"]}"#
}

fn body_for_add_shared_team() -> &str {
    r#"{"users":["github:crates-test-org:owners"]}"#
}

fn new_req(app: Arc<App>, krate: &str, version: &str) -> MockRequest {
    new_req_full(app, ::krate(krate), version, Vec::new())
}

fn new_req_full(app: Arc<App>, krate: Crate, version: &str,
                deps: Vec<u::CrateDependency>) -> MockRequest {
    let mut req = ::req(app, Method::Put, "/api/v1/crates/new");
    req.with_body(&new_req_body(krate, version, deps));
    return req;
}

fn new_req_body(krate: Crate, version: &str, deps: Vec<u::CrateDependency>)
                -> Vec<u8> {
    let kws = krate.keywords.into_iter().map(u::Keyword).collect();
    new_crate_to_body(&u::NewCrate {
        name: u::CrateName(krate.name),
        vers: u::CrateVersion(semver::Version::parse(version).unwrap()),
        features: HashMap::new(),
        deps: deps,
        authors: vec!["foo".to_string()],
        description: Some("description".to_string()),
        homepage: krate.homepage,
        documentation: krate.documentation,
        readme: krate.readme,
        keywords: Some(u::KeywordList(kws)),
        license: Some("MIT".to_string()),
        license_file: None,
        repository: krate.repository,
    })
}

fn new_crate_to_body(new_crate: &u::NewCrate) -> Vec<u8> {
    let json = json::encode(&new_crate).unwrap();
    let mut body = Vec::new();
    body.extend([
        (json.len() >>  0) as u8,
        (json.len() >>  8) as u8,
        (json.len() >> 16) as u8,
        (json.len() >> 24) as u8,
    ].iter().cloned());
    body.extend(json.as_bytes().iter().cloned());
    body.extend([0, 0, 0, 0].iter().cloned());
    body
}






// Test adding team without `github:`
#[test]
fn not_github() {
    let (_b, app, middle) = ::app();
    let mut req = new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_both());
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
    let mut req = new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_both());
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
    let mut req = new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_both());
    ::mock_crate(&mut req, ::krate("foo"));

    let body = body_for_add_shared_team();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo/owners")
                           .with_method(Method::Put)
                           .with_body(body.as_bytes())));
}

// Test adding team as owner when not on in
#[test]
fn add_team_as_non_member() {
    let (_b, app, middle) = ::app();
    let mut req = new_req(app, "foo", "2.0.0");
    ::mock_user(&mut req, mock_user_on_one());
    ::mock_crate(&mut req, ::krate("foo"));

    let body = body_for_add_solo_team();
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

    let mut req = new_req(app.clone(), "foo", "1.0.0");
    ::mock_user(&mut req, mock_user_on_both());
    ::mock_crate(&mut req, ::krate("foo"));

    let body = body_for_add_solo_team();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo/owners")
                           .with_method(Method::Put)
                           .with_body(body.as_bytes())));

    ::mock_user(&mut req, mock_user_on_one());
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/new")
                           .with_body(&new_req_body(::krate("foo"), "2.0.0", vec![]))
                           .with_method(Method::Put)));
    assert!(json.errors[0].detail.contains("another user"),
            "{:?}", json.errors);
}

// Test trying to publish a krate we do own (but only because of teams)
#[test]
fn publish_owned() {
    let (_b, app, middle) = ::app();
    let mut req = new_req(app.clone(), "foo", "1.0.0");
    ::mock_user(&mut req, mock_user_on_both());
    ::mock_crate(&mut req, ::krate("foo"));

    let body = body_for_add_shared_team();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo/owners")
                           .with_method(Method::Put)
                           .with_body(body.as_bytes())));

    ::mock_user(&mut req, mock_user_on_one());
    ok_resp!(middle.call(req.with_path("/api/v1/crates/new")
                           .with_body(&new_req_body(::krate("foo"), "2.0.0", vec![]))
                           .with_method(Method::Put)));
}

// Test trying to change owners (when only on an owning team)
#[test]
fn change_owners() {
    let (_b, app, middle) = ::app();
    let mut req = new_req(app.clone(), "foo", "1.0.0");
    ::mock_user(&mut req, mock_user_on_both());
    ::mock_crate(&mut req, ::krate("foo"));

    let body = body_for_add_shared_team();
    ok_resp!(middle.call(req.with_path("/api/v1/crates/foo/owners")
                           .with_method(Method::Put)
                           .with_body(body.as_bytes())));

    ::mock_user(&mut req, mock_user_on_one());
    let body = r#"{"users":["FlashCat"]}"#;     // User doesn't matter
    let json = bad_resp!(middle.call(req.with_path("/api/v1/crates/foo/owners")
                                       .with_method(Method::Put)
                                       .with_body(body.as_bytes())));
    assert!(json.errors[0].detail.contains("don't have permission"),
            "{:?}", json.errors);
}

