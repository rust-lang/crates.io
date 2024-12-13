use crate::app::AppState;
use axum::response::IntoResponse;
use axum_extra::json;

/// Get crates.io metadata.
///
/// Returns the current deployed commit SHA1 (or `unknown`), and whether the
/// system is in read-only mode.
#[utoipa::path(
    get,
    path = "/api/v1/site_metadata",
    operation_id = "get_site_metadata",
    tag = "other",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn show_deployed_sha(state: AppState) -> impl IntoResponse {
    let read_only = state.config.db.are_all_read_only();

    let deployed_sha =
        dotenvy::var("HEROKU_SLUG_COMMIT").unwrap_or_else(|_| String::from("unknown"));

    json!({
        "deployed_sha": &deployed_sha[..],
        "commit": &deployed_sha[..],
        "read_only": read_only,
    })
}
