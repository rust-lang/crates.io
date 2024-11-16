use crate::app::AppState;
use axum::response::IntoResponse;
use axum_extra::json;

/// Returns the JSON representation of the current deployed commit sha.
///
/// The sha is contained within the `HEROKU_SLUG_COMMIT` environment variable.
/// If `HEROKU_SLUG_COMMIT` is not set, returns `"unknown"`.
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
