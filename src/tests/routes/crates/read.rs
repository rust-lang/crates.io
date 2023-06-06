use crate::builders::{CrateBuilder, PublishBuilder, VersionBuilder};
use crate::util::{RequestHelper, TestApp};
use diesel::prelude::*;

#[test]
fn show() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let krate = app.db(|conn| {
        use crates_io::schema::versions;
        use diesel::{update, ExpressionMethods};

        let krate = CrateBuilder::new("foo_show", user.id)
            .description("description")
            .documentation("https://example.com")
            .homepage("http://example.com")
            .version(VersionBuilder::new("1.0.0"))
            .version(VersionBuilder::new("0.5.0"))
            .version(VersionBuilder::new("0.5.1"))
            .keyword("kw1")
            .downloads(20)
            .recent_downloads(10)
            .expect_build(conn);

        // Make version 1.0.0 mimic a version published before we started recording who published
        // versions
        let none: Option<i32> = None;
        update(versions::table)
            .filter(versions::num.eq("1.0.0"))
            .set(versions::published_by.eq(none))
            .execute(conn)
            .unwrap();

        krate
    });

    let json = anon.show_crate("foo_show");
    assert_eq!(json.krate.name, krate.name);
    assert_eq!(json.krate.id, krate.name);
    assert_eq!(json.krate.description, krate.description);
    assert_eq!(json.krate.homepage, krate.homepage);
    assert_eq!(json.krate.documentation, krate.documentation);
    assert_eq!(json.krate.keywords, Some(vec!["kw1".into()]));
    assert_eq!(json.krate.recent_downloads, Some(10));
    let crate_versions = json.krate.versions.as_ref().unwrap();
    assert_eq!(crate_versions.len(), 3);
    let versions = json.versions.as_ref().unwrap();
    assert_eq!(versions.len(), 3);

    assert_eq!(versions[0].id, crate_versions[0]);
    assert_eq!(versions[0].krate, json.krate.id);
    assert_eq!(versions[0].num, "1.0.0");
    assert_none!(&versions[0].published_by);
    let suffix = "/api/v1/crates/foo_show/1.0.0/download";
    assert!(
        versions[0].dl_path.ends_with(suffix),
        "bad suffix {}",
        versions[0].dl_path
    );
    let keywords = json.keywords.as_ref().unwrap();
    assert_eq!(1, keywords.len());
    assert_eq!("kw1", keywords[0].id);

    assert_eq!(versions[1].num, "0.5.1");
    assert_eq!(versions[2].num, "0.5.0");
    assert_eq!(
        versions[1].published_by.as_ref().unwrap().login,
        user.gh_login
    );
}

#[test]
fn show_minimal() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let krate = app.db(|conn| {
        use crates_io::schema::versions;
        use diesel::{update, ExpressionMethods};

        let krate = CrateBuilder::new("foo_show_minimal", user.id)
            .description("description")
            .documentation("https://example.com")
            .homepage("http://example.com")
            .version(VersionBuilder::new("1.0.0"))
            .version(VersionBuilder::new("0.5.0"))
            .version(VersionBuilder::new("0.5.1"))
            .keyword("kw1")
            .downloads(20)
            .recent_downloads(10)
            .expect_build(conn);

        // Make version 1.0.0 mimic a version published before we started recording who published
        // versions
        let none: Option<i32> = None;
        update(versions::table)
            .filter(versions::num.eq("1.0.0"))
            .set(versions::published_by.eq(none))
            .execute(conn)
            .unwrap();

        krate
    });

    let json = anon.show_crate_minimal("foo_show_minimal");
    assert_eq!(json.krate.name, krate.name);
    assert_eq!(json.krate.id, krate.name);
    assert_eq!(json.krate.description, krate.description);
    assert_eq!(json.krate.homepage, krate.homepage);
    assert_eq!(json.krate.documentation, krate.documentation);
    assert_eq!(json.krate.keywords, None);
    assert_eq!(json.krate.recent_downloads, None);
    assert_eq!(json.krate.versions, None);
    assert!(json.versions.is_none());
    assert!(json.keywords.is_none());
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
fn block_bad_documentation_url() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("foo_bad_doc_url", user.id)
            .documentation("http://rust-ci.org/foo/foo_bad_doc_url/doc/foo_bad_doc_url/")
            .expect_build(conn)
    });

    let json = anon.show_crate("foo_bad_doc_url");
    assert_eq!(json.krate.documentation, None);
}
