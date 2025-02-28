use crate::app::AppState;
use crate::controllers::version::CrateVersionPath;
use crate::util::{RequestUtils, redirect};
use axum::Json;
use axum::response::{IntoResponse, Response};
use http::request::Parts;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UrlResponse {
    /// The URL to the readme file.
    #[schema(example = "https://static.crates.io/readmes/serde/serde-1.0.0.html")]
    pub url: String,
}

/// Get the readme of a crate version.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/{version}/readme",
    params(CrateVersionPath),
    tag = "versions",
    responses(
        (status = 302, description = "Successful Response (default)", headers(("location" = String, description = "The URL to the readme file."))),
        (status = 200, description = "Successful Response (for `content-type: application/json`)", body = inline(UrlResponse)),
    ),
)]
pub async fn get_version_readme(app: AppState, path: CrateVersionPath, req: Parts) -> Response {
    let url = app.storage.readme_location(&path.name, &path.version);
    if req.wants_json() {
        Json(UrlResponse { url }).into_response()
    } else {
        redirect(url)
    }
}
