use crate::{
    schema::users,
    tests::{
        builders::{CrateBuilder, VersionBuilder},
        util::{RequestHelper, TestApp},
    },
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn admin_list_by_a_non_admin_fails() {
    let (_app, anon, user) = TestApp::init().with_user().await;

    let response = anon.admin_list("anything").await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(
        response.text(),
        @r#"{"errors":[{"detail":"this action requires authentication"}]}"#
    );

    let response = user.admin_list("anything").await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(
        response.text(),
        @r#"{"errors":[{"detail":"must be an admin to use this route"}]}"#
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn index_include_yanked() -> anyhow::Result<()> {
    let (app, _anon, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let user = user.as_model();

    let admin = app.db_new_user("admin").await;

    diesel::update(admin.as_model())
        .set(users::is_admin.eq(true))
        .execute(&mut conn)
        .await
        .unwrap();

    let crate_1 = CrateBuilder::new("unyanked", user.id)
        .version(VersionBuilder::new("0.1.0").yanked(true))
        .version(VersionBuilder::new("1.0.0"))
        .version(VersionBuilder::new("2.0.0"))
        .expect_build(&mut conn)
        .await;

    CrateBuilder::new("all_yanked", user.id)
        .version(VersionBuilder::new("1.0.0").yanked(true))
        .version(VersionBuilder::new("2.0.0").yanked(true))
        .expect_build(&mut conn)
        .await;

    CrateBuilder::new("someone_elses_crate", admin.as_model().id)
        .version(VersionBuilder::new("1.0.0").dependency(&crate_1, None))
        .expect_build(&mut conn)
        .await;

    // Include fully yanked (all versions were yanked) crates
    let username = &user.gh_login;
    let json = admin.admin_list(username).await.good();

    assert_eq!(json.user_email.unwrap(), "foo@example.com");
    assert_eq!(json.crates.len(), 2);

    assert_eq!(json.crates[0].name, "all_yanked");
    assert_eq!(json.crates[0].num_versions, 2);
    assert_eq!(json.crates[0].num_rev_deps, 0);

    assert_eq!(json.crates[1].name, "unyanked");
    assert_eq!(json.crates[1].num_versions, 3);
    assert_eq!(json.crates[1].num_rev_deps, 1);

    Ok(())
}
