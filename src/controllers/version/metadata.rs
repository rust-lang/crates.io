//! Endpoints that expose metadata about crate versions
//!
//! These endpoints provide data that could be obtained directly from the
//! index or cached metadata which was extracted (client side) from the
//! `Cargo.toml` file.

use crate::app::AppState;
use crate::models::VersionOwnerAction;
use crate::util::errors::AppResult;
use crate::views::EncodableVersion;
use axum::Json;
use serde::Serialize;

use super::CrateVersionPath;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GetResponse {
    pub version: EncodableVersion,
}

/// Get crate version metadata.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/{version}",
    params(CrateVersionPath),
    tag = "versions",
    responses((status = 200, description = "Successful Response", body = inline(GetResponse))),
)]
pub async fn find_version(state: AppState, path: CrateVersionPath) -> AppResult<Json<GetResponse>> {
    let mut conn = state.db_read().await?;
    let (version, krate) = path.load_version_and_crate(&mut conn).await?;
    let (actions, published_by) = tokio::try_join!(
        VersionOwnerAction::by_version(&mut conn, &version),
        version.published_by(&mut conn),
    )?;

    let version = EncodableVersion::from(version, &krate.name, published_by, actions);
    Ok(Json(GetResponse { version }))
}
