use std::sync::Arc;

extern crate diesel;
extern crate serde_json;

use serde_json::Value;

use conduit::{Handler, Method};
use self::diesel::prelude::*;

use views::{EncodableVersion, EncodableVersionBuildInfo};
use schema::versions;

#[derive(Deserialize)]
struct VersionList {
    versions: Vec<EncodableVersion>,
}
#[derive(Deserialize)]
struct VersionResponse {
    version: EncodableVersion,
}

#[test]
fn index() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(Arc::clone(&app), Method::Get, "/api/v1/versions");
    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionList = ::json(&mut response);
    assert_eq!(json.versions.len(), 0);

    let (v1, v2) = {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        ::CrateBuilder::new("foo_vers_index", u.id)
            .version(::VersionBuilder::new("2.0.0").license(Some("MIT")))
            .version(::VersionBuilder::new("2.0.1").license(Some("MIT/Apache-2.0")))
            .expect_build(&conn);
        let ids = versions::table
            .select(versions::id)
            .load::<i32>(&*conn)
            .unwrap();
        (ids[0], ids[1])
    };
    req.with_query(&format!("ids[]={}&ids[]={}", v1, v2));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionList = ::json(&mut response);
    assert_eq!(json.versions.len(), 2);

    for v in &json.versions {
        match v.num.as_ref() {
            "2.0.0" => assert_eq!(v.license, Some(String::from("MIT"))),
            "2.0.1" => assert_eq!(v.license, Some(String::from("MIT/Apache-2.0"))),
            _ => panic!("unexpected version"),
        }
    }
}

#[test]
fn show() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(Arc::clone(&app), Method::Get, "/api/v1/versions");
    let v = {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        let krate = ::CrateBuilder::new("foo_vers_show", user.id).expect_build(&conn);
        ::new_version(krate.id, "2.0.0").save(&conn, &[]).unwrap()
    };
    req.with_path(&format!("/api/v1/versions/{}", v.id));
    let mut response = ok_resp!(middle.call(&mut req));
    let json: VersionResponse = ::json(&mut response);
    assert_eq!(json.version.id, v.id);
}

#[test]
fn authors() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(
        Arc::clone(&app),
        Method::Get,
        "/api/v1/crates/foo_authors/1.0.0/authors",
    );
    {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        let c = ::CrateBuilder::new("foo_authors", u.id).expect_build(&conn);
        ::new_version(c.id, "1.0.0").save(&conn, &[]).unwrap();
    }
    let mut response = ok_resp!(middle.call(&mut req));
    let mut data = Vec::new();
    response.body.write_body(&mut data).unwrap();
    let s = ::std::str::from_utf8(&data).unwrap();
    let json: Value = serde_json::from_str(s).unwrap();
    let json = json.as_object().unwrap();
    assert!(json.contains_key(&"users".to_string()));
}

#[test]
fn record_rerendered_readme_time() {
    let (_b, app, _middle) = ::app();
    let version = {
        let conn = app.diesel_database.get().unwrap();
        let u = ::new_user("foo").create_or_update(&conn).unwrap();
        let c = ::CrateBuilder::new("foo_authors", u.id).expect_build(&conn);
        ::new_version(c.id, "1.0.0").save(&conn, &[]).unwrap()
    };
    {
        let conn = app.diesel_database.get().unwrap();
        version.record_readme_rendering(&conn).unwrap();
        version.record_readme_rendering(&conn).unwrap();
    }
}

#[test]
fn publish_build_info() {
    #[derive(Deserialize)]
    struct O {
        ok: bool,
    }
    let (_b, app, middle) = ::app();

    let mut req = ::new_req(Arc::clone(&app), "publish-build-info", "1.0.0");

    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::CrateBuilder::new("publish-build-info", user.id)
            .version("1.0.0")
            .expect_build(&conn);
        ::sign_in_as(&mut req, &user);
    }

    let body = r#"{
        "name":"publish-build-info",
        "vers":"1.0.0",
        "rust_version":"rustc 1.16.0-nightly (df8debf6d 2017-01-25)",
        "target":"x86_64-pc-windows-gnu",
        "passed":false}"#;

    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/publish-build-info/1.0.0/build_info")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(::json::<O>(&mut response).ok);

    let body = r#"{
        "name":"publish-build-info",
        "vers":"1.0.0",
        "rust_version":"rustc 1.16.0-nightly (df8debf6d 2017-01-25)",
        "target":"x86_64-pc-windows-gnu",
        "passed":true}"#;

    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/publish-build-info/1.0.0/build_info")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(::json::<O>(&mut response).ok);

    let body = r#"{
        "name":"publish-build-info",
        "vers":"1.0.0",
        "rust_version":"rustc 1.13.0 (df8debf6d 2017-01-25)",
        "target":"x86_64-pc-windows-gnu",
        "passed":true}"#;

    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/publish-build-info/1.0.0/build_info")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(::json::<O>(&mut response).ok);

    let body = r#"{
        "name":"publish-build-info",
        "vers":"1.0.0",
        "rust_version":"rustc 1.15.0-beta (df8debf6d 2017-01-20)",
        "target":"x86_64-pc-windows-gnu",
        "passed":true}"#;

    let mut response = ok_resp!(
        middle.call(
            req.with_path("/api/v1/crates/publish-build-info/1.0.0/build_info")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );
    assert!(::json::<O>(&mut response).ok);

    let mut response = ok_resp!(middle.call(req.with_path(
        "/api/v1/crates/publish-build-info/1.0.0/build_info"
    ).with_method(Method::Get)));

    #[derive(Deserialize)]
    struct R {
        build_info: EncodableVersionBuildInfo,
    }

    let json = ::json::<R>(&mut response);

    let nightly_key_string = String::from("2017-01-25T00:00:00+00:00");
    assert_eq!(
        json.build_info.ordering.get("nightly"),
        Some(&vec![nightly_key_string.clone()])
    );
    assert_eq!(
        json.build_info
            .nightly
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec![nightly_key_string]
    );

    let beta_key_string = String::from("2017-01-20T00:00:00+00:00");
    assert_eq!(
        json.build_info.ordering.get("beta"),
        Some(&vec![beta_key_string.clone()])
    );
    assert_eq!(
        json.build_info
            .beta
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec![beta_key_string]
    );

    let stable_key_string = String::from("1.13.0");
    assert_eq!(
        json.build_info.ordering.get("stable"),
        Some(&vec![stable_key_string.clone()])
    );
    assert_eq!(
        json.build_info
            .stable
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec![stable_key_string]
    );
}

#[test]
fn bad_rust_version_publish_build_info() {
    let (_b, app, middle) = ::app();

    let mut req = ::new_req(Arc::clone(&app), "bad-rust-vers", "1.0.0");

    {
        let conn = app.diesel_database.get().unwrap();
        let user = ::new_user("foo").create_or_update(&conn).unwrap();
        ::CrateBuilder::new("bad-rust-vers", user.id)
            .version("1.0.0")
            .expect_build(&conn);
        ::sign_in_as(&mut req, &user);
    }

    let body = r#"{
        "name":"bad-rust-vers",
        "vers":"1.0.0",
        "rust_version":"rustc 1.16.0-dev (df8debf6d 2017-01-25)",
        "target":"x86_64-pc-windows-gnu",
        "passed":true}"#;

    let response = bad_resp!(
        middle.call(
            req.with_path("/api/v1/crates/bad-rust-vers/1.0.0/build_info")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );

    assert_eq!(
        response.errors[0].detail,
        "invalid upload request: rust_version `rustc 1.16.0-dev (df8debf6d 2017-01-25)` not \
         recognized as nightly, beta, or stable at line 4 column 64"
    );

    let body = r#"{
        "name":"bad-rust-vers",
        "vers":"1.0.0",
        "rust_version":"1.15.0",
        "target":"x86_64-pc-windows-gnu",
        "passed":true}"#;

    let response = bad_resp!(
        middle.call(
            req.with_path("/api/v1/crates/bad-rust-vers/1.0.0/build_info")
                .with_method(Method::Put)
                .with_body(body.as_bytes()),
        )
    );

    assert_eq!(
        response.errors[0].detail,
        "invalid upload request: rust_version `1.15.0` not recognized; expected format like `rustc X.Y.Z (SHA YYYY-MM-DD)` at line 4 column 31"
    );
}
