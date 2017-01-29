use std::collections::HashMap;
use rustc_serialize::json::Json;

use conduit::{Handler, Request, Method};
use semver;

use cargo_registry::db::RequestTransaction;
use cargo_registry::version::{EncodableVersion, Version};

#[derive(RustcDecodable)]
struct VersionList { versions: Vec<EncodableVersion> }
#[derive(RustcDecodable)]
struct VersionResponse { version: EncodableVersion }

fn sv(s: &str) -> semver::Version {
    semver::Version::parse(s).unwrap()
}

#[test]
fn index() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/versions");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionList = ::json(&mut response);
    assert_eq!(json.versions.len(), 0);

    let (v1, v2) = {
        ::mock_user(&mut req, ::user("foo"));
        let (c, _) = ::mock_crate(&mut req, ::krate("foo_vers_index"));
        let req: &mut Request = &mut req;
        let tx = req.tx().unwrap();
        let m = HashMap::new();
        let v1 = Version::insert(tx, c.id, &sv("2.0.0"), &m, &[]).unwrap();
        let v2 = Version::insert(tx, c.id, &sv("2.0.1"), &m, &[]).unwrap();
        (v1, v2)
    };
    req.with_query(&format!("ids[]={}&ids[]={}", v1.id, v2.id));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionList = ::json(&mut response);
    assert_eq!(json.versions.len(), 2);
}

#[test]
fn show() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/versions");
    let v = {
        ::mock_user(&mut req, ::user("foo"));
        let (krate, _) = ::mock_crate(&mut req, ::krate("foo_vers_show"));
        let req: &mut Request = &mut req;
        let tx = req.tx().unwrap();
        Version::insert(tx, krate.id, &sv("2.0.0"), &HashMap::new(), &[]).unwrap()
    };
    req.with_path(&format!("/api/v1/versions/{}", v.id));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionResponse = ::json(&mut response);
    assert_eq!(json.version.id, v.id);
}

#[test]
fn authors() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/crates/foo_authors/1.0.0/authors");
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo_authors"));
    let mut response = ok_resp!(middle.call(&mut req));
    let mut s = String::new();
    response.body.read_to_string(&mut s).unwrap();
    let json = Json::from_str(&s).unwrap();
    let json = json.as_object().unwrap();
    assert!(json.contains_key(&"users".to_string()));
}

#[test]
fn publish_build_info() {
    #[derive(RustcDecodable)] struct O { ok: bool }
    let (_b, app, middle) = ::app();

    let mut req = ::new_req(app.clone(), "publish-build-info", "1.0.0");
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("publish-build-info"));

    let body = "{\
        \"name\":\"publish-build-info\",\
        \"vers\":\"1.0.0\",\
        \"rust_version\":\"rustc 1.16.0-nightly (df8debf6d 2017-01-25)\",\
        \"target\":\"x86_64-pc-windows-gnu\",\
        \"passed\":true}";

    let mut response = ok_resp!(middle.call(req.with_path(
        "/api/v1/crates/publish-build-info/1.0.0/build_info")
        .with_method(Method::Put)
        .with_body(body.as_bytes())));
    assert!(::json::<O>(&mut response).ok);

    let body = "{\
        \"name\":\"publish-build-info\",\
        \"vers\":\"1.0.0\",\
        \"rust_version\":\"rustc 1.13.0 (df8debf6d 2017-01-25)\",\
        \"target\":\"x86_64-pc-windows-gnu\",\
        \"passed\":true}";

    let mut response = ok_resp!(middle.call(req.with_path(
        "/api/v1/crates/publish-build-info/1.0.0/build_info")
        .with_method(Method::Put)
        .with_body(body.as_bytes())));
    assert!(::json::<O>(&mut response).ok);

    let body = "{\
        \"name\":\"publish-build-info\",\
        \"vers\":\"1.0.0\",\
        \"rust_version\":\"rustc 1.15.0-beta (df8debf6d 2017-01-20)\",\
        \"target\":\"x86_64-pc-windows-gnu\",\
        \"passed\":true}";

    let mut response = ok_resp!(middle.call(req.with_path(
        "/api/v1/crates/publish-build-info/1.0.0/build_info")
        .with_method(Method::Put)
        .with_body(body.as_bytes())));
    assert!(::json::<O>(&mut response).ok);
}

#[test]
fn bad_rust_version_publish_build_info() {
    let (_b, app, middle) = ::app();

    let mut req = ::new_req(app.clone(), "bad-rust-vers", "1.0.0");
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("bad-rust-vers"));

    let body = "{\
        \"name\":\"bad-rust-vers\",\
        \"vers\":\"1.0.0\",\
        \"rust_version\":\"rustc 1.16.0-dev (df8debf6d 2017-01-25)\",\
        \"target\":\"x86_64-pc-windows-gnu\",\
        \"passed\":true}";

    let response = bad_resp!(middle.call(req.with_path(
        "/api/v1/crates/bad-rust-vers/1.0.0/build_info")
        .with_method(Method::Put)
        .with_body(body.as_bytes())));

    assert_eq!(
        response.errors[0].detail,
        "rust_version `rustc 1.16.0-dev (df8debf6d 2017-01-25)` \
         not recognized as nightly, beta, or stable");

    let body = "{\
        \"name\":\"bad-rust-vers\",\
        \"vers\":\"1.0.0\",\
        \"rust_version\":\"1.15.0\",\
        \"target\":\"x86_64-pc-windows-gnu\",\
        \"passed\":true}";

    let response = bad_resp!(middle.call(req.with_path(
        "/api/v1/crates/bad-rust-vers/1.0.0/build_info")
        .with_method(Method::Put)
        .with_body(body.as_bytes())));

    assert_eq!(
        response.errors[0].detail,
        "rust_version `1.15.0` not recognized; \
        expected format like `rustc X.Y.Z (SHA YYYY-MM-DD)`");
}
