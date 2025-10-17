use crate::tests::builders::{CrateBuilder, VersionBuilder};
use crate::tests::util::insta::{self, assert_json_snapshot};
use crate::tests::util::{RequestHelper, TestApp};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::Value;

#[tokio::test(flavor = "multi_thread")]
async fn show_by_crate_name_and_version() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let krate = CrateBuilder::new("foo_vers_show", user.id)
        .expect_build(&mut conn)
        .await;
    let v = VersionBuilder::new("2.0.0")
        .size(1234)
        .checksum("c241cd77c3723ccf1aa453f169ee60c0a888344da504bee0142adb859092acb4")
        .rust_version("1.64")
        .expect_build(krate.id, user.id, &mut conn)
        .await;

    let url = "/api/v1/crates/foo_vers_show/2.0.0";
    let json: Value = anon.get(url).await.good();
    assert_json_snapshot!(json, {
        ".version.id" => insta::id_redaction(v.id),
        ".version.created_at" => "[datetime]",
        ".version.updated_at" => "[datetime]",
        ".version.published_by.id" => insta::id_redaction(user.id),
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn show_by_crate_name_and_semver_no_published_by() {
    use crate::schema::versions;
    use diesel::update;

    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let krate = CrateBuilder::new("foo_vers_show_no_pb", user.id)
        .expect_build(&mut conn)
        .await;
    let version = VersionBuilder::new("1.0.0")
        .expect_build(krate.id, user.id, &mut conn)
        .await;

    // Mimic a version published before we started recording who published versions
    let none: Option<i32> = None;
    update(versions::table)
        .set(versions::published_by.eq(none))
        .execute(&mut conn)
        .await
        .unwrap();

    let url = "/api/v1/crates/foo_vers_show_no_pb/1.0.0";
    let json: Value = anon.get(url).await.good();
    assert_json_snapshot!(json, {
        ".version.id" => insta::id_redaction(version.id),
        ".version.created_at" => "[datetime]",
        ".version.updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn block_bad_version_urls() {
    use crate::schema::versions;
    use diesel::{ExpressionMethods, update};

    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let _krate = CrateBuilder::new("foo_bad_version_urls", user.id)
        .version(VersionBuilder::new("1.0.0"))
        .expect_build(&mut conn)
        .await;

    update(versions::table)
        .filter(versions::num.eq("1.0.0"))
        .set((
            versions::homepage.eq(Some("http://rust-ci.org/foo/homepage")),
            versions::documentation.eq(Some("http://rust-ci.org/foo/docs")),
            versions::repository.eq(Some("http://rust-ci.org/foo/repo")),
        ))
        .execute(&mut conn)
        .await
        .unwrap();

    let url = "/api/v1/crates/foo_bad_version_urls/1.0.0";
    let json: Value = anon.get(url).await.good();
    assert_eq!(json["version"]["homepage"], Value::Null);
    assert_eq!(json["version"]["documentation"], Value::Null);
    assert_eq!(json["version"]["repository"], Value::Null);
}
