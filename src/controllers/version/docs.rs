//! Endpoint for triggering a docs.rs rebuild

use super::CrateVersionPath;
use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::util::errors::{AppResult, forbidden};
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
    if app.config.docs_rs_api_token.is_none() {
        return Err(forbidden("docs.rs integration is not configured"));
    };

    let mut conn = app.db_write().await?;
    AuthCheck::only_cookie().check(&req, &mut conn).await?;

    // validate if version & crate exist
    path.load_version_and_crate(&mut conn).await?;

    if let Err(error) = jobs::DocsRsQueueRebuild::new(path.name, path.version)
        .enqueue(&mut conn)
        .await
    {
        error!(
            ?error,
            "docs_rs_queue_rebuild: Failed to enqueue background job"
        );
    }

    Ok(StatusCode::CREATED.into_response())
}
