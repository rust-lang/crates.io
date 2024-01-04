use crate::builders::{CrateBuilder, VersionBuilder};
use crate::util::{RequestHelper, TestApp};
use crates_io::schema::versions;
use crates_io::views::EncodableVersion;
use diesel::{prelude::*, update};

#[derive(Deserialize)]
struct VersionsList {
    versions: Vec<EncodableVersion>,
}

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

    let json: VersionsList = anon.get("/api/v1/crates/foo_versions/versions").good();

    assert_eq!(json.versions.len(), 3);
    assert_eq!(json.versions[0].num, "1.0.0");
    assert_some_eq!(&json.versions[0].rust_version, "1.64");
    assert_eq!(json.versions[1].num, "0.5.1");
    assert_eq!(json.versions[2].num, "0.5.0");
    assert_none!(&json.versions[0].published_by);
    assert_eq!(
        json.versions[1].published_by.as_ref().unwrap().login,
        user.gh_login
    );
}
