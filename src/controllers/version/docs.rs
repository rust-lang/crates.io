//! Endpoint for triggering a docs.rs rebuild

use super::CrateVersionPath;
use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::controllers::helpers::authorization::Rights;
use crate::util::errors::{AppResult, custom, server_error};
use crate::worker::jobs;
use crates_io_worker::BackgroundJob as _;
use http::StatusCode;
use http::request::Parts;

/// Trigger a rebuild for the crate documentation on docs.rs.
#[utoipa::path(
    post,
    path = "/api/v1/crates/{name}/{version}/rebuild_docs",
    params(CrateVersionPath),
    security(
        ("cookie" = []),
    ),
    tag = "versions",
    responses((status = 201, description = "Successful Response")),
)]
pub async fn rebuild_version_docs(
    app: AppState,
    path: CrateVersionPath,
    req: Parts,
) -> AppResult<StatusCode> {
    let mut conn = app.db_write().await?;
    let auth = AuthCheck::only_cookie().check(&req, &mut conn).await?;

    // validate if version & crate exist
    let (_, krate) = path.load_version_and_crate(&mut conn).await?;

    // Check that the user is an owner of the crate, or a team member (= publish rights)
    let user = auth.user();
    let owners = krate.owners(&mut conn).await?;
    if Rights::get(user, &*app.github, &owners).await? < Rights::Publish {
        return Err(custom(
            StatusCode::FORBIDDEN,
            "user doesn't have permission to trigger a docs rebuild",
        ));
    }

    let job = jobs::DocsRsQueueRebuild::new(path.name, path.version);
    job.enqueue(&mut conn).await.map_err(|error| {
        error!("docs_rs_queue_rebuild: Failed to create background job: {error}");
        server_error("failed to create background job")
    })?;

    Ok(StatusCode::CREATED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        builders::{CrateBuilder, VersionBuilder},
        util::{RequestHelper as _, TestApp},
    };
    use crates_io_database::models::NewUser;
    use crates_io_docs_rs::MockDocsRsClient;

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
        assert_eq!(response.status(), StatusCode::CREATED);

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
            .gh_access_token("token")
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
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

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

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        app.run_pending_background_jobs().await;

        Ok(())
    }
}
