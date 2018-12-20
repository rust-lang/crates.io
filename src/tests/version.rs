use crate::{
    builders::{CrateBuilder, PublishBuilder, VersionBuilder},
    RequestHelper, TestApp, VersionResponse,
};
use cargo_registry::{schema::versions, views::EncodableVersion};

use diesel::prelude::*;
use serde_json::Value;

#[derive(Deserialize)]
struct VersionList {
    versions: Vec<EncodableVersion>,
}

#[test]
fn index() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    let url = "/api/v1/versions";

    let json: VersionList = anon.get(url).good();
    assert_eq!(json.versions.len(), 0);

    let (v1, v2) = app.db(|conn| {
        CrateBuilder::new("foo_vers_index", user.id)
            .version(VersionBuilder::new("2.0.0").license(Some("MIT")))
            .version(VersionBuilder::new("2.0.1").license(Some("MIT/Apache-2.0")))
            .expect_build(conn);
        let ids = versions::table
            .select(versions::id)
            .load::<i32>(conn)
            .unwrap();
        (ids[0], ids[1])
    });

    let query = format!("ids[]={}&ids[]={}", v1, v2);
    let json: VersionList = anon.get_with_query(url, &query).good();
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
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let v = app.db(|conn| {
        let krate = CrateBuilder::new("foo_vers_show", user.id).expect_build(conn);
        VersionBuilder::new("2.0.0")
            .size(1234)
            .expect_build(krate.id, conn)
    });

    let url = format!("/api/v1/versions/{}", v.id);
    let json: VersionResponse = anon.get(&url).good();
    assert_eq!(json.version.id, v.id);
    assert_eq!(json.version.crate_size, Some(1234));
}

#[test]
fn authors() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("foo_authors", user.id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let json: Value = anon.get("/api/v1/crates/foo_authors/1.0.0/authors").good();
    let json = json.as_object().unwrap();
    assert!(json.contains_key("users"));
}

#[test]
fn record_rerendered_readme_time() {
    let (app, _, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c = CrateBuilder::new("foo_authors", user.id).expect_build(conn);
        let version = VersionBuilder::new("1.0.0").expect_build(c.id, conn);

        version.record_readme_rendering(conn).unwrap();
        version.record_readme_rendering(conn).unwrap();
    });
}

#[test]
fn version_size() {
    let (_, _, user) = TestApp::with_proxy().with_user();
    let crate_to_publish = PublishBuilder::new("foo_version_size").version("1.0.0");
    user.publish(crate_to_publish).good();

    // Add a file to version 2 so that it's a different size than version 1
    let files = [("foo_version_size-2.0.0/big", &[b'a'; 1] as &[_])];
    let crate_to_publish = PublishBuilder::new("foo_version_size")
        .version("2.0.0")
        .files(&files);
    user.publish(crate_to_publish).good();

    let crate_json = user.show_crate("foo_version_size");

    let version1 = crate_json
        .versions
        .iter()
        .find(|v| v.num == "1.0.0")
        .expect("Could not find v1.0.0");
    assert_eq!(version1.crate_size, Some(35));

    let version2 = crate_json
        .versions
        .iter()
        .find(|v| v.num == "2.0.0")
        .expect("Could not find v2.0.0");
    assert_eq!(version2.crate_size, Some(91));
}
