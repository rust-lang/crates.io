use crate::builders::{CrateBuilder, VersionBuilder};
use crate::util::{RequestHelper, TestApp};
use diesel::prelude::*;

#[test]
fn show() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let krate = app.db(|conn| {
        use cargo_registry::schema::versions;
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
    let versions = json.krate.versions.as_ref().unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(json.versions.len(), 3);

    assert_eq!(json.versions[0].id, versions[0]);
    assert_eq!(json.versions[0].krate, json.krate.id);
    assert_eq!(json.versions[0].num, "1.0.0");
    assert_none!(&json.versions[0].published_by);
    let suffix = "/api/v1/crates/foo_show/1.0.0/download";
    assert!(
        json.versions[0].dl_path.ends_with(suffix),
        "bad suffix {}",
        json.versions[0].dl_path
    );
    assert_eq!(1, json.keywords.len());
    assert_eq!("kw1", json.keywords[0].id);

    assert_eq!(json.versions[1].num, "0.5.1");
    assert_eq!(json.versions[2].num, "0.5.0");
    assert_eq!(
        json.versions[1].published_by.as_ref().unwrap().login,
        user.gh_login
    );
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
