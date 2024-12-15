//! Endpoints for yanking and unyanking specific versions of crates

use super::metadata::{authenticate, perform_version_yank_update};
use super::version_and_crate;
use crate::app::AppState;
use crate::controllers::helpers::ok_true;
use crate::rate_limiter::LimitedAction;
use crate::util::errors::{version_not_found, AppResult};
use axum::extract::Path;
use axum::response::Response;
use http::request::Parts;

/// Yank a crate version.
///
/// This does not delete a crate version, it makes the crate
/// version accessible only to crates that already have a
/// `Cargo.lock` containing this version.
///
/// Notes:
///
/// Version deletion is not implemented to avoid breaking builds,
/// and the goal of yanking a crate is to prevent crates
/// beginning to depend on the yanked crate version.
#[utoipa::path(
    delete,
    path = "/api/v1/crates/{name}/{version}/yank",
    tag = "versions",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn yank_version(
    app: AppState,
    Path((crate_name, version)): Path<(String, String)>,
    req: Parts,
) -> AppResult<Response> {
    modify_yank(crate_name, version, app, req, true).await
}

/// Unyank a crate version.
#[utoipa::path(
    put,
    path = "/api/v1/crates/{name}/{version}/unyank",
    tag = "versions",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn unyank_version(
    app: AppState,
    Path((crate_name, version)): Path<(String, String)>,
    req: Parts,
) -> AppResult<Response> {
    modify_yank(crate_name, version, app, req, false).await
}

/// Changes `yanked` flag on a crate version record
async fn modify_yank(
    crate_name: String,
    version: String,
    state: AppState,
    req: Parts,
    yanked: bool,
) -> AppResult<Response> {
    // FIXME: Should reject bad requests before authentication, but can't due to
    // lifetime issues with `req`.

    if semver::Version::parse(&version).is_err() {
        return Err(version_not_found(&crate_name, &version));
    }

    let mut conn = state.db_write().await?;
    let (mut version, krate) = version_and_crate(&mut conn, &crate_name, &version).await?;
    let auth = authenticate(&req, &mut conn, &crate_name).await?;

    state
        .rate_limiter
        .check_rate_limit(auth.user_id(), LimitedAction::YankUnyank, &mut conn)
        .await?;

    perform_version_yank_update(
        &state,
        &mut conn,
        &mut version,
        &krate,
        &auth,
        Some(yanked),
        None,
    )
    .await?;

    ok_true()
}
