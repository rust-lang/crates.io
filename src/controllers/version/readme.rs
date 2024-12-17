use crate::app::AppState;
use crate::controllers::version::CrateVersionPath;
use crate::util::{redirect, RequestUtils};
use axum::response::{IntoResponse, Response};
use axum_extra::json;
use http::request::Parts;

/// Get the readme of a crate version.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/{version}/readme",
    params(CrateVersionPath),
    tag = "versions",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn get_version_readme(app: AppState, path: CrateVersionPath, req: Parts) -> Response {
    let redirect_url = app.storage.readme_location(&path.name, &path.version);
    if req.wants_json() {
        json!({ "url": redirect_url }).into_response()
    } else {
        redirect(redirect_url)
    }
}
