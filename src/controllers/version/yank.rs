//! Endpoints for yanking and unyanking specific versions of crates

use super::metadata::{authenticate, perform_version_yank_update};
use super::version_and_crate;
use crate::app::AppState;
use crate::controllers::helpers::ok_true;
use crate::tasks::spawn_blocking;
use crate::util::errors::{version_not_found, AppResult};
use axum::extract::Path;
use axum::response::Response;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use http::request::Parts;

/// Handles the `DELETE /crates/:crate_id/:version/yank` route.
/// This does not delete a crate version, it makes the crate
/// version accessible only to crates that already have a
/// `Cargo.lock` containing this version.
///
/// Notes:
/// Crate deletion is not implemented to avoid breaking builds,
/// and the goal of yanking a crate is to prevent crates
/// beginning to depend on the yanked crate version.
pub async fn yank(
    app: AppState,
    Path((crate_name, version)): Path<(String, String)>,
    req: Parts,
) -> AppResult<Response> {
    modify_yank(crate_name, version, app, req, true).await
}

/// Handles the `PUT /crates/:crate_id/:version/unyank` route.
pub async fn unyank(
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
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();
        let auth = authenticate(&req, conn, &crate_name)?;
        perform_version_yank_update(
            &state,
            conn,
            &mut version,
            &krate,
            &auth,
            Some(yanked),
            None,
        )?;
        ok_true()
    })
    .await
}
