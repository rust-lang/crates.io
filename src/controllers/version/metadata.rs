//! Endpoints that expose metadata about crate versions
//!
//! These endpoints provide data that could be obtained directly from the
//! index or cached metadata which was extracted (client side) from the
//! `Cargo.toml` file.

use crate::controllers::frontend_prelude::*;

use crate::models::VersionOwnerAction;
use crate::util::errors::version_not_found;
use crate::views::{EncodableDependency, EncodableVersion};

use super::version_and_crate;

/// Handles the `GET /crates/:crate_id/:version/dependencies` route.
///
/// This information can be obtained directly from the index.
///
/// In addition to returning cached data from the index, this returns
/// fields for `id`, `version_id`, and `downloads` (which appears to always
/// be 0)
pub async fn dependencies(
    state: AppState,
    Path((crate_name, version)): Path<(String, String)>,
) -> AppResult<Json<Value>> {
    if semver::Version::parse(&version).is_err() {
        return Err(version_not_found(&crate_name, &version));
    }

    let conn = state.db_read().await?;
    conn.interact(move |conn| {
        let (version, _) = version_and_crate(conn, &crate_name, &version)?;
        let deps = version.dependencies(conn)?;
        let deps = deps
            .into_iter()
            .map(|(dep, crate_name)| EncodableDependency::from_dep(dep, &crate_name))
            .collect::<Vec<_>>();

        Ok(Json(json!({ "dependencies": deps })))
    })
    .await?
}

/// Handles the `GET /crates/:crate_id/:version/authors` route.
pub async fn authors() -> Json<Value> {
    // Currently we return the empty list.
    // Because the API is not used anymore after RFC https://github.com/rust-lang/rfcs/pull/3052.

    Json(json!({
        "users": [],
        "meta": { "names": [] },
    }))
}

/// Handles the `GET /crates/:crate/:version` route.
///
/// The frontend doesn't appear to hit this endpoint, but our tests do, and it seems to be a useful
/// API route to have.
pub async fn show(
    state: AppState,
    Path((crate_name, version)): Path<(String, String)>,
) -> AppResult<Json<Value>> {
    if semver::Version::parse(&version).is_err() {
        return Err(version_not_found(&crate_name, &version));
    }

    let conn = state.db_read().await?;
    conn.interact(move |conn| {
        let (version, krate) = version_and_crate(conn, &crate_name, &version)?;
        let published_by = version.published_by(conn);
        let actions = VersionOwnerAction::by_version(conn, &version)?;

        let version = EncodableVersion::from(version, &krate.name, published_by, actions);
        Ok(Json(json!({ "version": version })))
    })
    .await?
}
