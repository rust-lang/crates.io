use std::io::{mod, fs, File, MemWriter};
use std::io::fs::PathExtensions;
use std::collections::HashMap;
use serialize::{json, Decoder, Decodable};

use conduit::{mod, Handler, Request};
use conduit_test::MockRequest;
use git2;
use semver;

use cargo_registry::db::RequestTransaction;
use cargo_registry::krate::{EncodableCrate, Crate};
use cargo_registry::version::EncodableVersion;
use cargo_registry::upload as u;

#[deriving(Decodable)]
struct CrateList { crates: Vec<EncodableCrate>, meta: CrateMeta }
#[deriving(Decodable)]
struct CrateMeta { total: int }
#[deriving(Decodable)]
struct BadCrate { ok: bool, error: String }
#[deriving(Decodable)]
struct GitCrate { name: String, vers: String, deps: Vec<String>, cksum: String }
struct GoodCrate { ok: bool, krate: EncodableCrate }
struct CrateResponse { krate: EncodableCrate, versions: Vec<EncodableVersion> }

impl<E, D: Decoder<E>> Decodable<D, E> for CrateResponse {
    fn decode(d: &mut D) -> Result<CrateResponse, E> {
        d.read_struct("CrateResponse", 2, |d| {
            Ok(CrateResponse {
                krate: try!(d.read_struct_field("crate", 0, Decodable::decode)),
                versions: try!(d.read_struct_field("versions", 1,
                                                   Decodable::decode)),
            })
        })
    }
}

impl<E, D: Decoder<E>> Decodable<D, E> for GoodCrate {
    fn decode(d: &mut D) -> Result<GoodCrate, E> {
        d.read_struct("GoodCrate", 2, |d| {
            Ok(GoodCrate {
                ok: try!(d.read_struct_field("ok", 0, Decodable::decode)),
                krate: try!(d.read_struct_field("crate", 1,
                                                Decodable::decode)),
            })
        })
    }
}

#[test]
fn index() {
    let (_b, _app, mut middle) = ::app();
    let mut req = MockRequest::new(conduit::Get, "/crates");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.crates.len(), 0);
    assert_eq!(json.meta.total, 0);

    let krate = ::krate();
    middle.add(::middleware::MockUser(::user()));
    middle.add(::middleware::MockCrate(krate.clone()));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.crates.len(), 1);
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.crates[0].name, krate.name);
    assert_eq!(json.crates[0].id, krate.name);
    assert_eq!(json.crates[0].versions.len(), 1);
}

#[test]
fn index_search() {
    let (_b, _app, mut middle) = ::app();
    let krate = ::krate();
    middle.add(::middleware::MockUser(::user()));
    middle.add(::middleware::MockCrate(krate.clone()));

    let mut req = MockRequest::new(conduit::Get, "/crates");
    req.with_query("q=bar");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.crates.len(), 0);
    assert_eq!(json.meta.total, 0);
    drop(req);

    let mut req = MockRequest::new(conduit::Get, "/crates");
    req.with_query("q=foo");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.crates.len(), 1);
    assert_eq!(json.meta.total, 1);
    drop(req);
}

#[test]
fn index_letter() {
    let (_b, _app, mut middle) = ::app();
    let krate = ::krate();
    middle.add(::middleware::MockUser(::user()));
    middle.add(::middleware::MockCrate(krate.clone()));

    let mut req = MockRequest::new(conduit::Get, "/crates");
    req.with_query("letter=B");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.crates.len(), 0);
    assert_eq!(json.meta.total, 0);
    drop(req);

    let mut req = MockRequest::new(conduit::Get, "/crates");
    req.with_query("letter=F");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CrateList = ::json(&mut response);
    assert_eq!(json.crates.len(), 1);
    assert_eq!(json.meta.total, 1);
    drop(req);
}

#[test]
fn show() {
    let (_b, _app, mut middle) = ::app();
    let krate = ::krate();
    middle.add(::middleware::MockUser(::user()));
    middle.add(::middleware::MockCrate(krate.clone()));
    let mut req = MockRequest::new(conduit::Get,
                                   format!("/crates/{}", krate.name).as_slice());
    let mut response = ok_resp!(middle.call(&mut req));
    let json: CrateResponse = ::json(&mut response);
    assert_eq!(json.krate.name, krate.name);
    assert_eq!(json.krate.id, krate.name);
    assert_eq!(json.krate.versions.len(), 1);
    assert_eq!(json.versions.len(), 1);
    assert_eq!(json.versions[0].id, json.krate.versions[0]);
    assert_eq!(json.versions[0].krate, json.krate.id);
    assert_eq!(json.versions[0].num, "1.0.0".to_string());
    let suffix = "/crates/foo/1.0.0/download";
    assert!(json.versions[0].dl_path.as_slice().ends_with(suffix),
            "bad suffix {}", json.versions[0].dl_path);
}

fn new_req(api_token: &str, krate: &str, version: &str,
           deps: Vec<u::CrateDependency>) -> MockRequest {
    let mut req = MockRequest::new(conduit::Put, "/crates/new");
    req.header("X-Cargo-Auth", api_token);

    let json = u::NewCrate {
        name: u::CrateName(krate.to_string()),
        vers: u::CrateVersion(semver::Version::parse(version).unwrap()),
        features: HashMap::new(),
        deps: deps,
    };
    let json = json::encode(&json);
    let mut body = MemWriter::new();
    println!("{} {}", json.len(), json);
    body.write_le_u32(json.len() as u32).unwrap();
    body.write_str(json.as_slice()).unwrap();
    body.write_le_u32(0).unwrap();
    req.with_body(body.unwrap().as_slice());
    return req;
}

#[test]
fn new_wrong_token() {
    let (_b, _app, mut middle) = ::app();
    middle.add(::middleware::MockUser(::user()));
    let mut req = new_req("wrong-token", "foo", "1.0.0", Vec::new());
    let mut response = ok_resp!(middle.call(&mut req));
    let json: BadCrate = ::json(&mut response);
    assert!(!json.ok);
}

#[test]
fn new_bad_names() {
    fn bad_name(name: &str) {
        let (_b, _app, mut middle) = ::app();
        let user = ::user();
        middle.add(::middleware::MockUser(user.clone()));
        let mut req = new_req(user.api_token.as_slice(), name, "1.0.0", Vec::new());
        let mut response = ok_resp!(middle.call(&mut req));
        let json: BadCrate = ::json(&mut response);
        assert!(!json.ok);
        assert!(json.error.as_slice().contains("invalid crate name"),
                "{}", json.error);
    }

    bad_name("");
    bad_name("foo bar");
}

#[test]
fn new_krate() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    middle.add(::middleware::MockUser(user.clone()));
    let mut req = new_req(user.api_token.as_slice(), "foo", "1.0.0", Vec::new());
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodCrate = ::json(&mut response);
    assert!(json.ok);
    assert_eq!(json.krate.name.as_slice(), "foo");
    assert_eq!(json.krate.max_version.as_slice(), "1.0.0");
}

#[test]
fn new_krate_twice() {
    let (_b, _app, mut middle) = ::app();
    let krate = ::krate();
    let user = ::user();
    middle.add(::middleware::MockUser(user.clone()));
    middle.add(::middleware::MockCrate(krate.clone()));
    let mut req = new_req(user.api_token.as_slice(),
                          krate.name.as_slice(),
                          "2.0.0", Vec::new());
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodCrate = ::json(&mut response);
    assert!(json.ok);
    assert_eq!(json.krate.name.as_slice(), krate.name.as_slice());
}

#[test]
fn new_krate_wrong_user() {
    let (_b, _app, mut middle) = ::app();

    // Crate will be owned by u2 (the last user)
    let mut u1 = ::user();
    u1.email = "some-new-email".to_string();
    let u2 = ::user();
    middle.add(::middleware::MockUser(u1.clone()));
    middle.add(::middleware::MockUser(u2));

    let krate = ::krate();
    middle.add(::middleware::MockCrate(krate.clone()));
    let mut req = new_req(u1.api_token.as_slice(),
                          krate.name.as_slice(),
                          "2.0.0", Vec::new());
    let mut response = t_resp!(middle.call(&mut req));
    let json: BadCrate = ::json(&mut response);
    assert!(!json.ok);
    assert!(json.error.as_slice().contains("another user"), "{}", json.error);
}

#[test]
fn new_krate_too_big() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    middle.add(::middleware::MockUser(user.clone()));
    let mut req = new_req(user.api_token.as_slice(), "foo", "1.0.0", Vec::new());
    req.with_body("a".repeat(1000 * 1000).as_slice());
    let mut response = ok_resp!(middle.call(&mut req));
    let json: BadCrate = ::json(&mut response);
    assert!(!json.ok);
}

#[test]
fn new_krate_duplicate_version() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    let krate = ::krate();
    middle.add(::middleware::MockUser(user.clone()));
    middle.add(::middleware::MockCrate(krate.clone()));
    let mut req = new_req(user.api_token.as_slice(),
                          krate.name.as_slice(),
                          "1.0.0", Vec::new());
    let mut response = ok_resp!(middle.call(&mut req));
    let json: BadCrate = ::json(&mut response);
    assert!(!json.ok);
    assert!(json.error.as_slice().contains("already uploaded"), "{}", json.error);
}

#[test]
fn new_krate_git_upload() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    middle.add(::middleware::MockUser(user.clone()));
    let mut req = new_req(user.api_token.as_slice(), "foo", "1.0.0", Vec::new());
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);

    let path = ::git::checkout().join("3/f/foo");
    assert!(path.exists());
    let contents = File::open(&path).read_to_string().unwrap();
    let p: GitCrate = json::decode(contents.as_slice()).unwrap();
    assert_eq!(p.name.as_slice(), "foo");
    assert_eq!(p.vers.as_slice(), "1.0.0");
    assert_eq!(p.deps.as_slice(), [].as_slice());
    assert_eq!(p.cksum.as_slice(),
               "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[test]
fn new_krate_git_upload_appends() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    let path = ::git::checkout().join("3/f/foo");
    fs::mkdir_recursive(&path.dir_path(), io::UserRWX).unwrap();
    File::create(&path).write_str(
        r#"{"name":"foo","vers":"0.0.1","deps":[],"cksum":"3j3"}"#
    ).unwrap();

    middle.add(::middleware::MockUser(user.clone()));
    let mut req = new_req(user.api_token.as_slice(), "foo", "1.0.0", Vec::new());
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);

    let contents = File::open(&path).read_to_string().unwrap();
    let mut lines = contents.as_slice().lines();
    let p1: GitCrate = json::decode(lines.next().unwrap()).unwrap();
    let p2: GitCrate = json::decode(lines.next().unwrap()).unwrap();
    assert!(lines.next().is_none());
    assert_eq!(p1.name.as_slice(), "foo");
    assert_eq!(p1.vers.as_slice(), "0.0.1");
    assert_eq!(p1.deps.as_slice(), [].as_slice());
    assert_eq!(p2.name.as_slice(), "foo");
    assert_eq!(p2.vers.as_slice(), "1.0.0");
    assert_eq!(p2.deps.as_slice(), [].as_slice());
}

#[test]
fn new_krate_git_upload_with_conflicts() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();

    {
        let repo = git2::Repository::open(&::git::bare()).unwrap();
        let target = repo.head().unwrap().target().unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.find_commit(target).unwrap();
        let tree = repo.find_tree(parent.tree_id()).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "empty commit", &tree,
                    &[&parent]).unwrap();
    }

    middle.add(::middleware::MockUser(user.clone()));
    let mut req = new_req(user.api_token.as_slice(), "foo", "1.0.0", Vec::new());
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodCrate>(&mut response);
}

#[test]
fn new_krate_dependency_missing() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    middle.add(::middleware::MockUser(user.clone()));
    let dep = u::CrateDependency {
        optional: false,
        default_features: true,
        name: u::CrateName("bar".to_string()),
        features: Vec::new(),
        version_req: u::CrateVersionReq(semver::VersionReq::parse(">= 0.0.0").unwrap()),
    };
    let mut req = new_req(user.api_token.as_slice(), "foo", "1.0.0", vec![dep]);
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<BadCrate>(&mut response);
}

#[test]
fn summary_doesnt_die() {
    let (_b, _app, middle) = ::app();
    let mut req = MockRequest::new(conduit::Get, "/summary");
    ok_resp!(middle.call(&mut req));
}

#[test]
fn download() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    let krate = ::krate();
    middle.add(::middleware::MockUser(user.clone()));
    middle.add(::middleware::MockCrate(krate.clone()));
    let rel = format!("/crates/{}/1.0.0/download", krate.name);
    let mut req = MockRequest::new(conduit::Get, rel.as_slice());
    let resp = t_resp!(middle.call(&mut req));
    assert_eq!(resp.status.val0(), 302);
    {
        let conn = (&mut req as &mut Request).tx().unwrap();
        let krate = Crate::find_by_name(conn, krate.name.as_slice()).unwrap();
        assert_eq!(krate.downloads, 0); // updated later
        let versions = krate.versions(conn).unwrap();
        assert_eq!(versions[0].downloads, 0); // updated later

        let stmt = conn.prepare("SELECT * FROM version_downloads").unwrap();
        let mut rows = stmt.query(&[]).unwrap();
        let row = rows.next().unwrap();
        assert!(rows.next().is_none());
        let downloads: i32 = row.get("downloads");
        assert_eq!(downloads, 1);
    }
}

#[test]
fn download_bad() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    let krate = ::krate();
    middle.add(::middleware::MockUser(user.clone()));
    middle.add(::middleware::MockCrate(krate.clone()));
    let rel = format!("/crates/{}/0.1.0/download", krate.name);
    let mut req = MockRequest::new(conduit::Get, rel.as_slice());
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<BadCrate>(&mut response);
}
