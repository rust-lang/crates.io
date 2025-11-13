use crate::app::AppState;
use axum::Json;
use axum::response::IntoResponse;
use crates_io_heroku::slug_commit;
use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MetadataResponse<'a> {
    /// The SHA1 of the currently deployed commit.
    #[schema(example = "0aebe2cdfacae1229b93853b1c58f9352195f081")]
    pub deployed_sha: &'a str,

    /// The SHA1 of the currently deployed commit.
    #[schema(example = "0aebe2cdfacae1229b93853b1c58f9352195f081")]
    pub commit: &'a str,

    /// Whether the crates.io service is in read-only mode.
    pub read_only: bool,

    /// Optional banner message to display on all pages.
    pub banner_message: Option<&'a str>,
}

/// Get crates.io metadata.
///
/// Returns the current deployed commit SHA1 (or `unknown`), and whether the
/// system is in read-only mode.
#[utoipa::path(
    get,
    path = "/api/v1/site_metadata",
    tag = "other",
    responses((status = 200, description = "Successful Response", body = inline(MetadataResponse<'_>))),
)]
pub async fn get_site_metadata(state: AppState) -> impl IntoResponse {
    let read_only = state.config.db.are_all_read_only();

    let deployed_sha = slug_commit().ok().flatten();
    let deployed_sha = deployed_sha.as_deref().unwrap_or("unknown");

    Json(MetadataResponse {
        deployed_sha,
        commit: deployed_sha,
        read_only,
        banner_message: state.config.banner_message.as_deref(),
    })
    .into_response()
}
