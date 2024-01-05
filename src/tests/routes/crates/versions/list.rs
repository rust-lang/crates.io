use crate::builders::{CrateBuilder, VersionBuilder};
use crate::util::{RequestHelper, TestApp};
use crates_io::schema::versions;
use diesel::{prelude::*, update};
use http::StatusCode;
use insta::{assert_display_snapshot, assert_json_snapshot};

#[test]
fn versions() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    app.db(|conn| {
        CrateBuilder::new("foo_versions", user.id)
            .version("0.5.1")
            .version(VersionBuilder::new("1.0.0").rust_version("1.64"))
            .version("0.5.0")
            .expect_build(conn);
        // Make version 1.0.0 mimic a version published before we started recording who published
        // versions
        let none: Option<i32> = None;
        update(versions::table)
            .filter(versions::num.eq("1.0.0"))
            .set(versions::published_by.eq(none))
            .execute(conn)
            .unwrap();
    });

    let response = anon.get::<()>("/api/v1/crates/foo_versions/versions");
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });
}

#[test]
fn test_unknown_crate() {
    let (_, anon) = TestApp::init().empty();

    let response = anon.get::<()>("/api/v1/crates/unknown/versions");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_display_snapshot!(response.text(), @r###"{"errors":[{"detail":"crate `unknown` does not exist"}]}"###);
}
