//! Endpoint for triggering a docs.rs rebuild

use super::CrateVersionPath;
use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::util::errors::{AppResult, server_error};
use crate::worker::jobs;
use axum::response::{IntoResponse as _, Response};
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
) -> AppResult<Response> {
    let mut conn = app.db_write().await?;
    AuthCheck::only_cookie().check(&req, &mut conn).await?;

    // validate if version & crate exist
    path.load_version_and_crate(&mut conn).await?;

    jobs::DocsRsQueueRebuild::new(path.name, path.version)
        .enqueue(&mut conn)
        .await
        .map_err(|error| {
            error!(
                ?error,
                "docs_rs_queue_rebuild: Failed to enqueue background job"
            );
            server_error("failed to enqueue background job")
        })?;

    Ok(StatusCode::CREATED.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        builders::{CrateBuilder, VersionBuilder},
        util::{RequestHelper as _, TestApp},
    };
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
