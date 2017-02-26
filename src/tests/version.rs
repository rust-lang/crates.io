use std::collections::HashMap;
use rustc_serialize::json::Json;

use conduit::{Handler, Request, Method};
use semver;
use time;

use cargo_registry::db::RequestTransaction;
use cargo_registry::version::{EncodableVersion, Version, EncodableBuildInfo};
use cargo_registry::upload;
use cargo_registry::krate::Crate;

#[derive(RustcDecodable)]
struct VersionList { versions: Vec<EncodableVersion> }
#[derive(RustcDecodable)]
struct VersionResponse { version: EncodableVersion }

fn sv(s: &str) -> semver::Version {
    semver::Version::parse(s).unwrap()
}

fn ts(s: &str) -> time::Timespec {
    time::strptime(s, "%Y-%m-%d").expect("Bad date string").to_timespec()
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

    let mut response = ok_resp!(middle.call(req.with_path(
        "/api/v1/crates/publish-build-info/1.0.0/build_info")
        .with_method(Method::Get)));

    #[derive(RustcDecodable)]
    struct R { build_info: EncodableBuildInfo }

    let json = ::json::<R>(&mut response);
    assert_eq!(
        json.build_info.ordering.get("nightly"),
        Some(&vec![String::from("2017-01-25T00:00:00Z")])
    );
    assert_eq!(
        json.build_info.ordering.get("beta"),
        Some(&vec![String::from("2017-01-20T00:00:00Z")])
    );
    assert_eq!(
        json.build_info.ordering.get("stable"),
        Some(&vec![String::from("1.13.0")])
    );
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

#[test]
fn no_existing_max_build_info() {
    let (_b, app, _middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/versions");
    ::mock_user(&mut req, ::user("foo"));
    let (krate, version) = ::mock_crate(&mut req, ::krate("no-existing-max"));
    let req: &mut Request = &mut req;
    let tx = req.tx().unwrap();

    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.14.0 (e8a012324 2016-12-16)"),
        target: String::from("anything"),
        passed: true,
    };
    version.store_build_info(tx, info, &krate).unwrap();

    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.15.0-beta.5 (10893a9a3 2017-01-19)"),
        target: String::from("anything"),
        passed: true,
    };
    version.store_build_info(tx, info, &krate).unwrap();

    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.16.0-nightly (df8debf6d 2017-01-25)"),
        target: String::from("anything"),
        passed: true,
    };
    version.store_build_info(tx, info, &krate).unwrap();

    let krate = Crate::find_by_name(tx, "no-existing-max").unwrap();
    assert_eq!(krate.max_build_info_stable, Some(sv("1.14.0")));
    assert_eq!(krate.max_build_info_beta, Some(ts("2017-01-19")));
    assert_eq!(krate.max_build_info_nightly, Some(ts("2017-01-25")));
}

#[test]
fn failed_build_info_doesnt_update_max() {
    let (_b, app, _middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/versions");
    ::mock_user(&mut req, ::user("foo"));
    let (krate, version) = ::mock_crate(&mut req, ::krate("failed-build"));
    let req: &mut Request = &mut req;
    let tx = req.tx().unwrap();

    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.14.0 (e8a012324 2016-12-16)"),
        target: String::from("anything"),
        passed: false, // this is the different part
    };
    version.store_build_info(tx, info, &krate).unwrap();

    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.15.0-beta.5 (10893a9a3 2017-01-19)"),
        target: String::from("anything"),
        passed: false, // this is the different part
    };
    version.store_build_info(tx, info, &krate).unwrap();

    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.16.0-nightly (df8debf6d 2017-01-25)"),
        target: String::from("anything"),
        passed: false, // this is the different part
    };
    version.store_build_info(tx, info, &krate).unwrap();

    let krate = Crate::find_by_name(tx, "failed-build").unwrap();
    assert_eq!(krate.max_build_info_stable, None);
    assert_eq!(krate.max_build_info_beta, None);
    assert_eq!(krate.max_build_info_nightly, None);
}

#[test]
fn not_max_version_build_info_doesnt_update_max() {
    let (_b, app, _middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/versions");
    ::mock_user(&mut req, ::user("foo"));
    let (krate, version) = ::mock_crate(&mut req, ::krate("old-rust-vers"));
    let req: &mut Request = &mut req;
    let tx = req.tx().unwrap();

    // Setup: store build info for:
    // stable 1.14.0, beta 2017-01-19, nightly 2017-01-25
    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.14.0 (e8a012324 2016-12-16)"),
        target: String::from("anything"),
        passed: true,
    };
    version.store_build_info(tx, info, &krate).unwrap();

    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.15.0-beta.5 (10893a9a3 2017-01-19)"),
        target: String::from("anything"),
        passed: true,
    };
    version.store_build_info(tx, info, &krate).unwrap();

    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.16.0-nightly (df8debf6d 2017-01-25)"),
        target: String::from("anything"),
        passed: true,
    };
    version.store_build_info(tx, info, &krate).unwrap();

    // Need to reload to see versions added in setup
    let krate = Crate::find_by_name(tx, "old-rust-vers").unwrap();

    // Report build info for:
    // stable 1.13.0, beta 2017-01-01, nightly 2017-01-24
    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.13.0 (2c6933acc 2016-11-07)"),
        target: String::from("anything"),
        passed: true,
    };
    version.store_build_info(tx, info, &krate).unwrap();

    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.15.0-beta.3 (beefcafe 2017-01-01)"),
        target: String::from("anything"),
        passed: true,
    };
    version.store_build_info(tx, info, &krate).unwrap();

    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.16.0-nightly (deadbeef 2017-01-24)"),
        target: String::from("anything"),
        passed: true,
    };
    version.store_build_info(tx, info, &krate).unwrap();

    // Max build info should still be 1.14.0, 2017-01-19, and 2017-01-25
    let krate = Crate::find_by_name(tx, "old-rust-vers").unwrap();
    assert_eq!(krate.max_build_info_stable, Some(sv("1.14.0")));
    assert_eq!(krate.max_build_info_beta, Some(ts("2017-01-19")));
    assert_eq!(krate.max_build_info_nightly, Some(ts("2017-01-25")));
}

#[test]
fn older_crate_version_in_build_info_doesnt_update_max() {
    let (_b, app, _middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/versions");
    ::mock_user(&mut req, ::user("foo"));

    let req: &mut Request = &mut req;

    let krate = ::krate("older-crate-version");

    // Publish a version of the crate
    let (krate, _) = ::mock_crate_vers(req, krate, &sv("2.0.0"));

    // Then go back and publish a lower version
    let (krate, version) = ::mock_crate(req, krate);

    assert_eq!(krate.max_version, sv("2.0.0"));
    assert_eq!(version.num, sv("1.0.0"));

    let tx = req.tx().unwrap();

    // Publish the build info for the lower version
    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.14.0 (e8a012324 2016-12-16)"),
        target: String::from("anything"),
        passed: true,
    };
    version.store_build_info(tx, info, &krate).unwrap();

    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.15.0-beta.5 (10893a9a3 2017-01-19)"),
        target: String::from("anything"),
        passed: true,
    };
    version.store_build_info(tx, info, &krate).unwrap();

    let info = upload::VersionBuildInfo {
        rust_version: String::from("rustc 1.16.0-nightly (df8debf6d 2017-01-25)"),
        target: String::from("anything"),
        passed: true,
    };
    version.store_build_info(tx, info, &krate).unwrap();

    let krate = Crate::find_by_name(tx, "older-crate-version").unwrap();
    assert_eq!(krate.max_build_info_stable, None);
    assert_eq!(krate.max_build_info_beta, None);
    assert_eq!(krate.max_build_info_nightly, None);
}

#[test]
fn clear_max_build_info_on_new_crate_max_version() {
    let (_b, app, _middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/versions");
    ::mock_user(&mut req, ::user("foo"));
    let (krate, version) = ::mock_crate(&mut req, ::krate("no-existing-max"));
    let req: &mut Request = &mut req;
    {
        let tx = req.tx().unwrap();

        // Setup: publish some build info for the current max version
        let info = upload::VersionBuildInfo {
            rust_version: String::from("rustc 1.14.0 (e8a012324 2016-12-16)"),
            target: String::from("anything"),
            passed: true,
        };
        version.store_build_info(tx, info, &krate).unwrap();

        let info = upload::VersionBuildInfo {
            rust_version: String::from("rustc 1.15.0-beta.5 (10893a9a3 2017-01-19)"),
            target: String::from("anything"),
            passed: true,
        };
        version.store_build_info(tx, info, &krate).unwrap();

        let info = upload::VersionBuildInfo {
            rust_version: String::from("rustc 1.16.0-nightly (df8debf6d 2017-01-25)"),
            target: String::from("anything"),
            passed: true,
        };
        version.store_build_info(tx, info, &krate).unwrap();
    }

    // Then publish a higher version of the crate
    ::mock_crate_vers(req, krate, &sv("2.0.0"));

    let tx = req.tx().unwrap();
    let krate = Crate::find_by_name(tx, "no-existing-max").unwrap();
    assert_eq!(krate.max_build_info_stable, None);
    assert_eq!(krate.max_build_info_beta, None);
    assert_eq!(krate.max_build_info_nightly, None);
}
