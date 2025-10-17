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
use tracing::error;

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
    let encryption = &app.config.gh_token_encryption;
    if Rights::get(user, &*app.github, &owners, encryption).await? < Rights::Publish {
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
