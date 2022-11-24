use crate::builders::{CrateBuilder, VersionBuilder};
use crate::util::{RequestHelper, TestApp};
use crate::CrateMeta;
use cargo_registry::views::{EncodableDependency, EncodableVersion};

#[derive(Deserialize)]
struct RevDeps {
    dependencies: Vec<EncodableDependency>,
    versions: Vec<EncodableVersion>,
    meta: CrateMeta,
}

impl crate::util::MockAnonymousUser {
    fn reverse_dependencies(&self, krate_name: &str) -> RevDeps {
        let url = format!("/api/v1/crates/{krate_name}/reverse_dependencies");
        self.get(&url).good()
    }
}

#[test]
fn reverse_dependencies() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id).expect_build(conn);
        CrateBuilder::new("c2", user.id)
            .version(VersionBuilder::new("1.0.0").dependency(&c1, None))
            .version(
                VersionBuilder::new("1.1.0")
                    .dependency(&c1, None)
                    .dependency(&c1, Some("foo")),
            )
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");
    assert_eq!(deps.versions.len(), 1);
    assert_eq!(deps.versions[0].krate, "c2");
    assert_eq!(deps.versions[0].num, "1.1.0");

    // c1 has no dependent crates.
    let deps = anon.reverse_dependencies("c2");
    assert_eq!(deps.dependencies.len(), 0);
    assert_eq!(deps.meta.total, 0);
}

#[test]
fn reverse_dependencies_when_old_version_doesnt_depend_but_new_does() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id)
            .version("1.1.0")
            .expect_build(conn);
        CrateBuilder::new("c2", user.id)
            .version("1.0.0")
            .version(VersionBuilder::new("2.0.0").dependency(&c1, None))
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");
}

#[test]
fn reverse_dependencies_when_old_version_depended_but_new_doesnt() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id)
            .version("1.0.0")
            .expect_build(conn);
        CrateBuilder::new("c2", user.id)
            .version(VersionBuilder::new("1.0.0").dependency(&c1, None))
            .version("2.0.0")
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 0);
    assert_eq!(deps.meta.total, 0);
}

#[test]
fn prerelease_versions_not_included_in_reverse_dependencies() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id)
            .version("1.0.0")
            .expect_build(conn);
        CrateBuilder::new("c2", user.id)
            .version("1.1.0-pre")
            .expect_build(conn);
        CrateBuilder::new("c3", user.id)
            .version(VersionBuilder::new("1.0.0").dependency(&c1, None))
            .version("1.1.0-pre")
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");
}

#[test]
fn yanked_versions_not_included_in_reverse_dependencies() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id)
            .version("1.0.0")
            .expect_build(conn);
        CrateBuilder::new("c2", user.id)
            .version("1.0.0")
            .version(VersionBuilder::new("2.0.0").dependency(&c1, None))
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");

    app.db(|conn| {
        use cargo_registry::schema::versions;
        use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

        diesel::update(versions::table.filter(versions::num.eq("2.0.0")))
            .set(versions::yanked.eq(true))
            .execute(conn)
            .unwrap();
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 0);
    assert_eq!(deps.meta.total, 0);
}

#[test]
fn reverse_dependencies_includes_published_by_user_when_present() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        use cargo_registry::schema::versions;
        use diesel::{update, ExpressionMethods, RunQueryDsl};

        let c1 = CrateBuilder::new("c1", user.id)
            .version("1.0.0")
            .expect_build(conn);
        CrateBuilder::new("c2", user.id)
            .version(VersionBuilder::new("2.0.0").dependency(&c1, None))
            .expect_build(conn);

        // Make c2's version (and,incidentally, c1's, but that doesn't matter) mimic a version
        // published before we started recording who published versions
        let none: Option<i32> = None;
        update(versions::table)
            .set(versions::published_by.eq(none))
            .execute(conn)
            .unwrap();

        // c3's version will have the published by info recorded
        CrateBuilder::new("c3", user.id)
            .version(VersionBuilder::new("3.0.0").dependency(&c1, None))
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.versions.len(), 2);

    let c2_version = deps.versions.iter().find(|v| v.krate == "c2").unwrap();
    assert_none!(&c2_version.published_by);

    let c3_version = deps.versions.iter().find(|v| v.krate == "c3").unwrap();
    assert_eq!(
        c3_version.published_by.as_ref().unwrap().login,
        user.gh_login
    );
}

#[test]
fn reverse_dependencies_query_supports_u64_version_number_parts() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let large_but_valid_version_number = format!("1.0.{}", std::u64::MAX);

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id).expect_build(conn);
        // The crate that depends on c1...
        CrateBuilder::new("c2", user.id)
            // ...has a patch version at the limits of what the semver crate supports
            .version(VersionBuilder::new(&large_but_valid_version_number).dependency(&c1, None))
            .expect_build(conn);
    });

    let deps = anon.reverse_dependencies("c1");
    assert_eq!(deps.dependencies.len(), 1);
    assert_eq!(deps.meta.total, 1);
    assert_eq!(deps.dependencies[0].crate_id, "c1");
    assert_eq!(deps.versions.len(), 1);
    assert_eq!(deps.versions[0].krate, "c2");
    assert_eq!(deps.versions[0].num, large_but_valid_version_number);
}
