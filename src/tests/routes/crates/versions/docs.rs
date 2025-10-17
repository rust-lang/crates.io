use crate::builders::{CrateBuilder, VersionBuilder};
use crate::util::{RequestHelper as _, TestApp};
use crates_io_database::models::NewUser;
use crates_io_docs_rs::MockDocsRsClient;
use insta::assert_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn test_trigger_rebuild_ok() -> anyhow::Result<()> {
    let mut docs_rs_mock = MockDocsRsClient::new();
    docs_rs_mock
        .expect_rebuild_docs()
        .returning(|_, _| Ok(()))
        .times(1);

    let (app, _client, cookie_client) =
        TestApp::full().with_docs_rs(docs_rs_mock).with_user().await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new("krate", cookie_client.as_model().id)
        .version(VersionBuilder::new("0.1.0"))
        .build(&mut conn)
        .await?;

    let response = cookie_client
        .post::<()>("/api/v1/crates/krate/0.1.0/rebuild_docs", "")
        .await;
    assert_snapshot!(response.status(), @"201 Created");

    app.run_pending_background_jobs().await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_trigger_rebuild_permission_failed() -> anyhow::Result<()> {
    let mut docs_rs_mock = MockDocsRsClient::new();
    docs_rs_mock
        .expect_rebuild_docs()
        .returning(|_, _| Ok(()))
        .never();

    let (app, _client, cookie_client) =
        TestApp::full().with_docs_rs(docs_rs_mock).with_user().await;

    let mut conn = app.db_conn().await;

    let other_user = NewUser::builder()
        .gh_id(111)
        .gh_login("other_user")
        .gh_encrypted_token(&[])
        .build()
        .insert(&mut conn)
        .await?;

    CrateBuilder::new("krate", other_user.id)
        .version(VersionBuilder::new("0.1.0"))
        .build(&mut conn)
        .await?;

    let response = cookie_client
        .post::<()>("/api/v1/crates/krate/0.1.0/rebuild_docs", "")
        .await;
    assert_snapshot!(response.status(), @"403 Forbidden");

    app.run_pending_background_jobs().await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_trigger_rebuild_unknown_crate_doesnt_queue_job() -> anyhow::Result<()> {
    let mut docs_rs_mock = MockDocsRsClient::new();
    docs_rs_mock
        .expect_rebuild_docs()
        .returning(|_, _| Ok(()))
        .never();

    let (app, _client, cookie_client) =
        TestApp::full().with_docs_rs(docs_rs_mock).with_user().await;

    let response = cookie_client
        .post::<()>("/api/v1/crates/krate/0.1.0/rebuild_docs", "")
        .await;

    assert_snapshot!(response.status(), @"404 Not Found");

    app.run_pending_background_jobs().await;

    Ok(())
}
