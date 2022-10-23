use crate::{
    builders::{CrateBuilder, PublishBuilder, VersionBuilder},
    RequestHelper, TestApp,
};
use cargo_registry::{models::Version, schema::versions};

use crate::util::insta::{self, assert_yaml_snapshot};
use diesel::prelude::*;
use serde_json::Value;

#[test]
fn index() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let url = "/api/v1/versions";

    let json: Value = anon.get(url).good();
    assert_yaml_snapshot!(json);

    let (v1, v2) = app.db(|conn| {
        CrateBuilder::new("foo_vers_index", user.id)
            .version(VersionBuilder::new("2.0.0").license(Some("MIT")))
            .version(VersionBuilder::new("2.0.1").license(Some("MIT/Apache-2.0")))
            .expect_build(conn);
        let ids: Vec<i32> = versions::table.select(versions::id).load(conn).unwrap();
        (ids[0], ids[1])
    });

    let query = format!("ids[]={v1}&ids[]={v2}");
    let json: Value = anon.get_with_query(url, &query).good();
    assert_yaml_snapshot!(json, {
        ".versions" => insta::sorted_redaction(),
        ".versions[].id" => insta::any_id_redaction(),
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
        ".versions[].published_by.id" => insta::id_redaction(user.id),
    });
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
    let json: Value = anon.get(&url).good();
    assert_yaml_snapshot!(json, {
        ".version.id" => insta::id_redaction(v.id),
        ".version.created_at" => "[datetime]",
        ".version.updated_at" => "[datetime]",
        ".version.published_by.id" => insta::id_redaction(user.id),
    });
}

#[test]
fn show_by_crate_name_and_version() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let v = app.db(|conn| {
        let krate = CrateBuilder::new("foo_vers_show", user.id).expect_build(conn);
        VersionBuilder::new("2.0.0")
            .size(1234)
            .checksum("c241cd77c3723ccf1aa453f169ee60c0a888344da504bee0142adb859092acb4")
            .expect_build(krate.id, user.id, conn)
    });

    let url = "/api/v1/crates/foo_vers_show/2.0.0";
    let json: Value = anon.get(url).good();
    assert_yaml_snapshot!(json, {
        ".version.id" => insta::id_redaction(v.id),
        ".version.created_at" => "[datetime]",
        ".version.updated_at" => "[datetime]",
        ".version.published_by.id" => insta::id_redaction(user.id),
    });
}

#[test]
fn show_by_crate_name_and_semver_no_published_by() {
    use diesel::update;

    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let v = app.db(|conn| {
        let krate = CrateBuilder::new("foo_vers_show_no_pb", user.id).expect_build(conn);
        let version = VersionBuilder::new("1.0.0").expect_build(krate.id, user.id, conn);

        // Mimic a version published before we started recording who published versions
        let none: Option<i32> = None;
        update(versions::table)
            .set(versions::published_by.eq(none))
            .execute(conn)
            .unwrap();

        version
    });

    let url = "/api/v1/crates/foo_vers_show_no_pb/1.0.0";
    let json: Value = anon.get(url).good();
    assert_yaml_snapshot!(json, {
        ".version.id" => insta::id_redaction(v.id),
        ".version.created_at" => "[datetime]",
        ".version.updated_at" => "[datetime]",
    });
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
    assert_yaml_snapshot!(json);
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
    user.publish_crate(crate_to_publish).good();

    // Add a file to version 2 so that it's a different size than version 1
    let files = [("foo_version_size-2.0.0/big", &[b'a'; 1] as &[_])];
    let crate_to_publish = PublishBuilder::new("foo_version_size")
        .version("2.0.0")
        .files(&files);
    user.publish_crate(crate_to_publish).good();

    let crate_json = user.show_crate("foo_version_size");

    let version1 = crate_json
        .versions
        .as_ref()
        .unwrap()
        .iter()
        .find(|v| v.num == "1.0.0")
        .expect("Could not find v1.0.0");
    assert_eq!(version1.crate_size, Some(35));

    let version2 = crate_json
        .versions
        .as_ref()
        .unwrap()
        .iter()
        .find(|v| v.num == "2.0.0")
        .expect("Could not find v2.0.0");
    assert_eq!(version2.crate_size, Some(91));
}

#[test]
fn daily_limit() {
    let (app, _, user) = TestApp::full().with_user();

    let max_daily_versions = app.as_inner().config.new_version_rate_limit.unwrap();
    for version in 1..=max_daily_versions {
        let crate_to_publish =
            PublishBuilder::new("foo_daily_limit").version(&format!("0.0.{}", version));
        user.publish_crate(crate_to_publish).good();
    }

    let crate_to_publish = PublishBuilder::new("foo_daily_limit").version("1.0.0");
    let response = user.publish_crate(crate_to_publish);
    assert!(response.status().is_success());
    let json = response.into_json();
    assert_eq!(
        json["errors"][0]["detail"],
        "You have published too many versions of this crate in the last 24 hours"
    );
}
