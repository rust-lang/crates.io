use crate::tests::builders::{CrateBuilder, VersionBuilder};
use crate::tests::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn reverse_dependencies() {
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

    let response = anon
        .get::<()>("/api/v1/crates/c1/reverse_dependencies")
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });

    // c1 has no dependent crates.
    let response = anon
        .get::<()>("/api/v1/crates/c2/reverse_dependencies")
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn reverse_dependencies_when_old_version_doesnt_depend_but_new_does() {
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

    let response = anon
        .get::<()>("/api/v1/crates/c1/reverse_dependencies")
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn reverse_dependencies_when_old_version_depended_but_new_doesnt() {
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

    let response = anon
        .get::<()>("/api/v1/crates/c1/reverse_dependencies")
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn prerelease_versions_not_included_in_reverse_dependencies() {
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

    let response = anon
        .get::<()>("/api/v1/crates/c1/reverse_dependencies")
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn yanked_versions_not_included_in_reverse_dependencies() {
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

    let response = anon
        .get::<()>("/api/v1/crates/c1/reverse_dependencies")
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });

    app.db(|conn| {
        use crate::schema::versions;
        use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

        diesel::update(versions::table.filter(versions::num.eq("2.0.0")))
            .set(versions::yanked.eq(true))
            .execute(conn)
            .unwrap();
    });

    let response = anon
        .get::<()>("/api/v1/crates/c1/reverse_dependencies")
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn reverse_dependencies_includes_published_by_user_when_present() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        use crate::schema::versions;
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

    let response = anon
        .get::<()>("/api/v1/crates/c1/reverse_dependencies")
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn reverse_dependencies_query_supports_u64_version_number_parts() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let large_but_valid_version_number = format!("1.0.{}", u64::MAX);

    app.db(|conn| {
        let c1 = CrateBuilder::new("c1", user.id).expect_build(conn);
        // The crate that depends on c1...
        CrateBuilder::new("c2", user.id)
            // ...has a patch version at the limits of what the semver crate supports
            .version(VersionBuilder::new(&large_but_valid_version_number).dependency(&c1, None))
            .expect_build(conn);
    });

    let response = anon
        .get::<()>("/api/v1/crates/c1/reverse_dependencies")
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_crate() {
    let (_, anon) = TestApp::init().empty();

    let response = anon
        .get::<()>("/api/v1/crates/unknown/reverse_dependencies")
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"crate `unknown` does not exist"}]}"###);
}
