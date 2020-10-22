use crate::{
    builders::{CrateBuilder, PublishBuilder, VersionBuilder},
    RequestHelper, TestApp, VersionResponse,
};
use cargo_registry::{models::Version, schema::versions, views::EncodableVersion};

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
fn show_by_id() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let v = app.db(|conn| {
        let krate = CrateBuilder::new("foo_vers_show_id", user.id).expect_build(conn);
        VersionBuilder::new("2.0.0")
            .size(1234)
            .expect_build(krate.id, user.id, conn)
    });

    let url = format!("/api/v1/versions/{}", v.id);
    let json: VersionResponse = anon.get(&url).good();
    assert_eq!(json.version.id, v.id);
    assert_eq!(json.version.crate_size, Some(1234));
}

#[test]
fn show_by_crate_name_and_semver_with_published_by() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let v = app.db(|conn| {
        let krate = CrateBuilder::new("foo_vers_show", user.id).expect_build(conn);
        VersionBuilder::new("2.0.0")
            .size(1234)
            .expect_build(krate.id, user.id, conn)
    });

    let json: VersionResponse = anon.show_version("foo_vers_show", "2.0.0");
    assert_eq!(json.version.id, v.id);
    assert_eq!(json.version.crate_size, Some(1234));
    assert_eq!(json.version.published_by.unwrap().login, user.gh_login);
}

#[test]
fn show_by_crate_name_and_semver_no_published_by() {
    use diesel::update;

    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("foo_vers_show_no_pb", user.id)
            .version("1.0.0")
            .expect_build(conn);
        // Mimic a version published before we started recording who published versions
        let none: Option<i32> = None;
        update(versions::table)
            .set(versions::published_by.eq(none))
            .execute(conn)
            .unwrap();
    });

    let json: VersionResponse = anon.show_version("foo_vers_show_no_pb", "1.0.0");
    assert!(json.version.published_by.is_none());
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
        let version = VersionBuilder::new("1.0.0").expect_build(c.id, user.id, conn);

        Version::record_readme_rendering(version.id, conn).unwrap();
        Version::record_readme_rendering(version.id, conn).unwrap();
    });
}

#[test]
fn version_size() {
    let (_, _, user) = TestApp::full().with_user();

    let crate_to_publish = PublishBuilder::new("foo_version_size").version("1.0.0");
    user.enqueue_publish(crate_to_publish).good();

    // Add a file to version 2 so that it's a different size than version 1
    let files = [("foo_version_size-2.0.0/big", &[b'a'; 1] as &[_])];
    let crate_to_publish = PublishBuilder::new("foo_version_size")
        .version("2.0.0")
        .files(&files);
    user.enqueue_publish(crate_to_publish).good();

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
