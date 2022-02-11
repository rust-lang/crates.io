use crate::builders::{CrateBuilder, VersionBuilder};
use crate::util::{RequestHelper, TestApp};
use diesel::prelude::*;

#[test]
fn krate() {
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

    let json = anon.show_crate_minimal("foo_show");
    assert_eq!(json.name, krate.name);
    assert_eq!(json.id, krate.name);
    assert_eq!(json.description, krate.description);
    assert_eq!(json.homepage, krate.homepage);
    assert_eq!(json.documentation, krate.documentation);
    assert_eq!(json.keywords, None);
    assert_eq!(json.recent_downloads, None);
    assert_eq!(json.versions, None);
}
