
use conduit::{mod, Handler};
use conduit_test::MockRequest;

#[deriving(Decodable)]
struct PackageList { packages: Vec<Package>, meta: PackageMeta }
#[deriving(Decodable)]
struct PackageMeta { total: int, page: int }
#[deriving(Decodable)]
struct Package { name: String, id: String }
#[deriving(Decodable)]
struct PackageResponse { package: Package }
#[deriving(Decodable)]
struct BadPackage { ok: bool, error: String }
#[deriving(Decodable)]
struct GoodPackage { ok: bool, package: Package }

#[test]
fn index() {
    let (_b, mut middle) = ::middleware();
    let mut req = MockRequest::new(conduit::Get, "/packages");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: PackageList = ::json(&mut response);
    assert_eq!(json.packages.len(), 0);
    assert_eq!(json.meta.total, 0);
    assert_eq!(json.meta.page, 0);

    let pkg = ::package();
    middle.add(::middleware::MockUser(::user()));
    middle.add(::middleware::MockPackage(pkg.clone()));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: PackageList = ::json(&mut response);
    assert_eq!(json.packages.len(), 1);
    assert_eq!(json.meta.total, 1);
    assert_eq!(json.meta.page, 0);
    assert_eq!(json.packages[0].name, pkg.name);
    assert_eq!(json.packages[0].id, pkg.name);
}

#[test]
fn show() {
    let (_b, mut middle) = ::middleware();
    let pkg = ::package();
    middle.add(::middleware::MockUser(::user()));
    middle.add(::middleware::MockPackage(pkg.clone()));
    let mut req = MockRequest::new(conduit::Get,
                                   format!("/packages/{}", pkg.name).as_slice());
    let mut response = ok_resp!(middle.call(&mut req));
    let json: PackageResponse = ::json(&mut response);
    assert_eq!(json.package.name, pkg.name);
    assert_eq!(json.package.id, pkg.name);
}

fn new_req(api_token: &str, pkg: &str, version: &str, deps: &[&str])
           -> MockRequest {
    let mut req = MockRequest::new(conduit::Post, "/packages/new");
    req.header("X-Cargo-Auth", api_token)
       .header("X-Cargo-Pkg-Name", pkg)
       .header("X-Cargo-Pkg-Version", version)
       .with_body("")
       .header("Content-Type", "application/x-tar")
       .header("Content-Encoding", "x-gzip");
    drop(deps);
    return req;
}

#[test]
fn new_wrong_token() {
    let (_b, mut middle) = ::middleware();
    middle.add(::middleware::MockUser(::user()));
    let mut req = new_req("wrong-token", "foo", "1.0.0", []);
    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.val0(), 404);
}

#[test]
fn new_bad_names() {
    fn bad_name(name: &str) {
        let (_b, mut middle) = ::middleware();
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
fn new_package() {
    let (_b, mut middle) = ::middleware();
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
    let (_b, mut middle) = ::middleware();
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
    let (_b, mut middle) = ::middleware();

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
