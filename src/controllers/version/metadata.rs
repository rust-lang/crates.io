//! Endpoints that expose metadata about crate versions
//!
//! These endpoints provide data that could be obtained direclty from the
//! index or cached metadata which was extracted (client side) from the
//! `Cargo.toml` file.

use crate::controllers::frontend_prelude::*;

use crate::models::VersionOwnerAction;
use crate::views::{EncodableDependency, EncodableVersion};

use super::{extract_crate_name_and_semver, version_and_crate};

/// Handles the `GET /crates/:crate_id/:version/dependencies` route.
///
/// This information can be obtained direclty from the index.
///
/// In addition to returning cached data from the index, this returns
/// fields for `id`, `version_id`, and `downloads` (which appears to always
/// be 0)
pub fn dependencies(req: &mut dyn RequestExt) -> EndpointResult {
    let (crate_name, semver) = extract_crate_name_and_semver(req)?;
    let conn = req.db_read_only()?;
    let (version, _) = version_and_crate(&conn, crate_name, semver)?;
    let deps = version.dependencies(&conn)?;
    let deps = deps
        .into_iter()
        .map(|(dep, crate_name)| EncodableDependency::from_dep(dep, &crate_name))
        .collect::<Vec<_>>();

    Ok(req.json(&json!({ "dependencies": deps })))
}

/// Handles the `GET /crates/:crate_id/:version/authors` route.
pub fn authors(req: &mut dyn RequestExt) -> EndpointResult {
    // Currently we return the empty list.
    // Because the API is not used anymore after RFC https://github.com/rust-lang/rfcs/pull/3052.

    Ok(req.json(&json!({
        "users": [],
        "meta": { "names": [] },
    })))
}

/// Handles the `GET /crates/:crate/:version` route.
///
/// The frontend doesn't appear to hit this endpoint, but our tests do, and it seems to be a useful
/// API route to have.
pub fn show(req: &mut dyn RequestExt) -> EndpointResult {
    let (crate_name, semver) = extract_crate_name_and_semver(req)?;
    let conn = req.db_read_only()?;
    let (version, krate) = version_and_crate(&conn, crate_name, semver)?;
    let published_by = version.published_by(&conn);
    let actions = VersionOwnerAction::by_version(&conn, &version)?;

    let version = EncodableVersion::from(version, &krate.name, published_by, actions);
    Ok(req.json(&json!({ "version": version })))
}
