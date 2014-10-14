use std::collections::HashMap;
use serialize::json;

use conduit::{mod, Handler, Request};
use semver;

use cargo_registry::db::RequestTransaction;
use cargo_registry::version::{EncodableVersion, Version};

#[deriving(Decodable)]
struct VersionList { versions: Vec<EncodableVersion> }
#[deriving(Decodable)]
struct VersionResponse { version: EncodableVersion }

fn sv(s: &str) -> semver::Version {
    semver::Version::parse(s).unwrap()
}

#[test]
fn index() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, conduit::Get, "/versions");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionList = ::json(&mut response);
    assert_eq!(json.versions.len(), 0);

    let (v1, v2) = {
        ::mock_user(&mut req, ::user("foo"));
        let c = ::mock_crate(&mut req, ::krate("foo"));
        let req = &mut req as &mut Request;
        let tx = req.tx().unwrap();
        let m = HashMap::new();
        let v1 = Version::insert(tx, c.id, &sv("2.0.0"), &m, []).unwrap();
        let v2 = Version::insert(tx, c.id, &sv("2.0.1"), &m, []).unwrap();
        (v1, v2)
    };
    req.with_query(format!("ids[]={}&ids[]={}", v1.id, v2.id));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionList = ::json(&mut response);
    assert_eq!(json.versions.len(), 2);
}

#[test]
fn show() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, conduit::Get, "/versions");
    let v = {
        ::mock_user(&mut req, ::user("foo"));
        let krate = ::mock_crate(&mut req, ::krate("foo"));
        let req = &mut req as &mut Request;
        let tx = req.tx().unwrap();
        Version::insert(tx, krate.id, &sv("2.0.0"), &HashMap::new(), []).unwrap()
    };
    req.with_path(format!("/versions/{}", v.id).as_slice());
    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionResponse = ::json(&mut response);
    assert_eq!(json.version.id, v.id);
}

#[test]
fn authors() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app, conduit::Get, "/crates/foo/1.0.0/authors");
    ::mock_user(&mut req, ::user("foo"));
    ::mock_crate(&mut req, ::krate("foo"));
    let mut response = ok_resp!(middle.call(&mut req));
    let s = response.body.read_to_string().unwrap();
    let json = json::from_str(s.as_slice()).unwrap();
    let json = json.as_object().unwrap();
    assert!(json.contains_key(&"users".to_string()));
}
