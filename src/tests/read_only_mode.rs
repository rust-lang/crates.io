use crate::tests::builders::CrateBuilder;
use crate::tests::{RequestHelper, TestApp};

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn can_hit_read_only_endpoints_in_read_only_mode() {
    let (_app, anon) = TestApp::init()
        .with_config(|config| {
            config.db.primary.read_only_mode = true;
        })
        .empty()
        .await;

    let response = anon.get::<()>("/api/v1/crates").await;
    assert_snapshot!(response.status(), @"200 OK");
}

#[tokio::test(flavor = "multi_thread")]
async fn cannot_hit_endpoint_which_writes_db_in_read_only_mode() {
    let (app, _, user, token) = TestApp::init()
        .with_config(|config| {
            config.db.primary.read_only_mode = true;
        })
        .with_token()
        .await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo_yank_read_only", user.as_model().id)
        .version("1.0.0")
        .expect_build(&mut conn)
        .await;

    let response = token
        .delete::<()>("/api/v1/crates/foo_yank_read_only/1.0.0/yank")
        .await;
    assert_snapshot!(response.status(), @"503 Service Unavailable");
    assert_json_snapshot!(response.json());
}

#[tokio::test(flavor = "multi_thread")]
async fn can_download_crate_in_read_only_mode() {
    let (app, anon, user) = TestApp::init()
        .with_config(|config| {
            config.db.primary.read_only_mode = true;
        })
        .with_user()
        .await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo_download_read_only", user.as_model().id)
        .version("1.0.0")
        .expect_build(&mut conn)
        .await;

    let response = anon
        .get::<()>("/api/v1/crates/foo_download_read_only/1.0.0/download")
        .await;
    assert_snapshot!(response.status(), @"302 Found");

    // We're in read only mode so the download should not have been counted

    use crate::schema::version_downloads;
    use diesel::dsl::sum;

    let dl_count: Result<Option<i64>, _> = version_downloads::table
        .select(sum(version_downloads::downloads))
        .get_result(&mut conn)
        .await;
    assert_ok_eq!(dl_count, None);
}
