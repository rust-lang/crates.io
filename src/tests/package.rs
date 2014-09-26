use std::io::{mod, fs, File};
use std::io::fs::PathExtensions;
use serialize::json;
use git2;

use conduit::{mod, Handler, Request};
use conduit_test::MockRequest;

use cargo_registry::db::RequestTransaction;
use cargo_registry::package::{EncodablePackage, Package};
use cargo_registry::version::EncodableVersion;

#[deriving(Decodable)]
struct PackageList { packages: Vec<EncodablePackage>, meta: PackageMeta }
#[deriving(Decodable)]
struct PackageMeta { total: int }
#[deriving(Decodable)]
struct PackageResponse { package: EncodablePackage, versions: Vec<EncodableVersion> }
#[deriving(Decodable)]
struct BadPackage { ok: bool, error: String }
#[deriving(Decodable)]
struct GoodPackage { ok: bool, package: EncodablePackage }
#[deriving(Decodable)]
struct GitPackage { name: String, vers: String, deps: Vec<String>, cksum: String }

#[test]
fn index() {
    let (_b, _app, mut middle) = ::app();
    let mut req = MockRequest::new(conduit::Get, "/packages");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: PackageList = ::json(&mut response);
    assert_eq!(json.packages.len(), 0);
    assert_eq!(json.meta.total, 0);

    let pkg = ::package();
    middle.add(::middleware::MockUser(::user()));
    middle.add(::middleware::MockPackage(pkg.clone()));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: PackageList = ::json(&mut response);
    assert_eq!(json.packages.len(), 1);
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.packages[0].name, pkg.name);
    assert_eq!(json.packages[0].id, pkg.name);
    assert_eq!(json.packages[0].versions.len(), 1);
}

#[test]
fn show() {
    let (_b, _app, mut middle) = ::app();
    let pkg = ::package();
    middle.add(::middleware::MockUser(::user()));
    middle.add(::middleware::MockPackage(pkg.clone()));
    let mut req = MockRequest::new(conduit::Get,
                                   format!("/packages/{}", pkg.name).as_slice());
    let mut response = ok_resp!(middle.call(&mut req));
    let json: PackageResponse = ::json(&mut response);
    assert_eq!(json.package.name, pkg.name);
    assert_eq!(json.package.id, pkg.name);
    assert_eq!(json.package.versions.len(), 1);
    assert_eq!(json.versions.len(), 1);
    assert_eq!(json.versions[0].id, json.package.versions[0]);
    assert_eq!(json.versions[0].pkg, json.package.id);
    assert_eq!(json.versions[0].num, "1.0.0".to_string());
    let suffix = "/download/foo/foo-1.0.0.tar.gz";
    assert!(json.versions[0].dl_path.as_slice().ends_with(suffix),
            "bad suffix {}", json.versions[0].dl_path);
}

fn new_req(api_token: &str, pkg: &str, version: &str, deps: &[&str])
           -> MockRequest {
    let mut req = MockRequest::new(conduit::Put, "/packages/new");
    req.header("X-Cargo-Auth", api_token)
       .header("X-Cargo-Pkg-Name", pkg)
       .header("X-Cargo-Pkg-Version", version)
       .header("X-Cargo-Pkg-Feature", "{}")
       .with_body("")
       .header("Content-Type", "application/x-tar")
       .header("Content-Encoding", "x-gzip");
    drop(deps);
    return req;
}

#[test]
fn new_wrong_token() {
    let (_b, _app, mut middle) = ::app();
    middle.add(::middleware::MockUser(::user()));
    let mut req = new_req("wrong-token", "foo", "1.0.0", []);
    let mut response = ok_resp!(middle.call(&mut req));
    let json: BadPackage = ::json(&mut response);
    assert!(!json.ok);
}

#[test]
fn new_bad_names() {
    fn bad_name(name: &str) {
        let (_b, _app, mut middle) = ::app();
        let user = ::user();
        middle.add(::middleware::MockUser(user.clone()));
        let mut req = new_req(user.api_token.as_slice(), name, "1.0.0", []);
        let mut response = ok_resp!(middle.call(&mut req));
        let json: BadPackage = ::json(&mut response);
        assert!(!json.ok);
        assert!(json.error.as_slice().contains("invalid package name"),
                "{}", json.error);
    }

    bad_name("");
    bad_name("foo bar");
}

#[test]
fn new_bad_versions() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    middle.add(::middleware::MockUser(user.clone()));
    let mut req = new_req(user.api_token.as_slice(), "foo", "1.0", []);
    let mut response = ok_resp!(middle.call(&mut req));
    let json: BadPackage = ::json(&mut response);
    assert!(!json.ok);
    assert!(json.error.as_slice().contains("invalid package version"),
            "{}", json.error);
}

#[test]
fn new_package() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    middle.add(::middleware::MockUser(user.clone()));
    let mut req = new_req(user.api_token.as_slice(), "foo", "1.0.0", []);
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodPackage = ::json(&mut response);
    assert!(json.ok);
    assert_eq!(json.package.name.as_slice(), "foo");
}

#[test]
fn new_package_twice() {
    let (_b, _app, mut middle) = ::app();
    let package = ::package();
    let user = ::user();
    middle.add(::middleware::MockUser(user.clone()));
    middle.add(::middleware::MockPackage(package.clone()));
    let mut req = new_req(user.api_token.as_slice(),
                          package.name.as_slice(),
                          "2.0.0", []);
    let mut response = ok_resp!(middle.call(&mut req));
    let json: GoodPackage = ::json(&mut response);
    assert!(json.ok);
    assert_eq!(json.package.name.as_slice(), package.name.as_slice());
}

#[test]
fn new_package_wrong_user() {
    let (_b, _app, mut middle) = ::app();

    // Package will be owned by u2 (the last user)
    let mut u1 = ::user();
    u1.email = "some-new-email".to_string();
    let u2 = ::user();
    middle.add(::middleware::MockUser(u1.clone()));
    middle.add(::middleware::MockUser(u2));

    let package = ::package();
    middle.add(::middleware::MockPackage(package.clone()));
    let mut req = new_req(u1.api_token.as_slice(),
                          package.name.as_slice(),
                          "2.0.0", []);
    let mut response = t_resp!(middle.call(&mut req));
    let json: BadPackage = ::json(&mut response);
    assert!(!json.ok);
    assert!(json.error.as_slice().contains("another user"), "{}", json.error);
}

#[test]
fn new_package_too_big() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    middle.add(::middleware::MockUser(user.clone()));
    let mut req = new_req(user.api_token.as_slice(), "foo", "1.0.0", []);
    req.with_body("a".repeat(1000 * 1000).as_slice());
    let mut response = ok_resp!(middle.call(&mut req));
    let json: BadPackage = ::json(&mut response);
    assert!(!json.ok);
}

#[test]
fn new_package_duplicate_version() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    let package = ::package();
    middle.add(::middleware::MockUser(user.clone()));
    middle.add(::middleware::MockPackage(package.clone()));
    let mut req = new_req(user.api_token.as_slice(),
                          package.name.as_slice(),
                          "1.0.0", []);
    let mut response = ok_resp!(middle.call(&mut req));
    let json: BadPackage = ::json(&mut response);
    assert!(!json.ok);
    assert!(json.error.as_slice().contains("already uploaded"), "{}", json.error);
}

#[test]
fn new_package_git_upload() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    middle.add(::middleware::MockUser(user.clone()));
    let mut req = new_req(user.api_token.as_slice(), "foo", "1.0.0", []);
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodPackage>(&mut response);

    let path = ::git::checkout().join("3/f/foo");
    assert!(path.exists());
    let contents = File::open(&path).read_to_string().unwrap();
    let p: GitPackage = json::decode(contents.as_slice()).unwrap();
    assert_eq!(p.name.as_slice(), "foo");
    assert_eq!(p.vers.as_slice(), "1.0.0");
    assert_eq!(p.deps.as_slice(), [].as_slice());
    assert_eq!(p.cksum.as_slice(),
               "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
}

#[test]
fn new_package_git_upload_appends() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    let path = ::git::checkout().join("3/f/foo");
    fs::mkdir_recursive(&path.dir_path(), io::UserRWX).unwrap();
    File::create(&path).write_str(
        r#"{"name":"foo","vers":"0.0.1","deps":[],"cksum":"3j3"}"#
    ).unwrap();

    middle.add(::middleware::MockUser(user.clone()));
    let mut req = new_req(user.api_token.as_slice(), "foo", "1.0.0", []);
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodPackage>(&mut response);

    let contents = File::open(&path).read_to_string().unwrap();
    let mut lines = contents.as_slice().lines();
    let p1: GitPackage = json::decode(lines.next().unwrap()).unwrap();
    let p2: GitPackage = json::decode(lines.next().unwrap()).unwrap();
    assert!(lines.next().is_none());
    assert_eq!(p1.name.as_slice(), "foo");
    assert_eq!(p1.vers.as_slice(), "0.0.1");
    assert_eq!(p1.deps.as_slice(), [].as_slice());
    assert_eq!(p2.name.as_slice(), "foo");
    assert_eq!(p2.vers.as_slice(), "1.0.0");
    assert_eq!(p2.deps.as_slice(), [].as_slice());
}

#[test]
fn new_package_git_upload_with_conflicts() {
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
    let mut req = new_req(user.api_token.as_slice(), "foo", "1.0.0", []);
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<GoodPackage>(&mut response);
}

#[test]
fn new_package_dependency_missing() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    middle.add(::middleware::MockUser(user.clone()));
    let mut req = new_req(user.api_token.as_slice(), "foo", "1.0.0", []);
    req.header("X-Cargo-Pkg-Dep", "bar||>=1.0.0");
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<BadPackage>(&mut response);
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
    let package = ::package();
    middle.add(::middleware::MockUser(user.clone()));
    middle.add(::middleware::MockPackage(package.clone()));
    let rel = format!("/{}/{}-1.0.0.tar.gz", package.name, package.name);
    let mut req = MockRequest::new(conduit::Get, format!("/download{}", rel)
                                                        .as_slice());
    let resp = t_resp!(middle.call(&mut req));
    assert_eq!(resp.status.val0(), 302);
    {
        let conn = (&mut req as &mut Request).tx().unwrap();
        let pkg = Package::find_by_name(conn, package.name.as_slice()).unwrap();
        assert_eq!(pkg.downloads, 1);
        let versions = pkg.versions(conn).unwrap();
        assert_eq!(versions[0].downloads, 1);
    }
}

#[test]
fn download_bad() {
    let (_b, _app, mut middle) = ::app();
    let user = ::user();
    let package = ::package();
    middle.add(::middleware::MockUser(user.clone()));
    middle.add(::middleware::MockPackage(package.clone()));
    let rel = format!("/{}/{}-0.1.0.tar.gz", package.name, package.name);
    let mut req = MockRequest::new(conduit::Get, format!("/download{}", rel)
                                                        .as_slice());
    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<BadPackage>(&mut response);
}
