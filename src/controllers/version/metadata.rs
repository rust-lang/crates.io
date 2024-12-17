//! Endpoints that expose metadata about crate versions
//!
//! These endpoints provide data that could be obtained directly from the
//! index or cached metadata which was extracted (client side) from the
//! `Cargo.toml` file.

use axum_extra::json;
use axum_extra::response::ErasedJson;

use crate::app::AppState;
use crate::models::VersionOwnerAction;
use crate::util::errors::AppResult;
use crate::views::EncodableVersion;

use super::CrateVersionPath;

/// Get crate version metadata.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/{version}",
    params(CrateVersionPath),
    tag = "versions",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn find_version(state: AppState, path: CrateVersionPath) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;
    let (version, krate) = path.load_version_and_crate(&mut conn).await?;
    let published_by = version.published_by(&mut conn).await?;
    let actions = VersionOwnerAction::by_version(&mut conn, &version).await?;

    let version = EncodableVersion::from(version, &krate.name, published_by, actions);
    Ok(json!({ "version": version }))
}
