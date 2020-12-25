//! Endpoints that expose metadata about crate versions
//!
//! These endpoints provide data that could be obtained direclty from the
//! index or cached metadata which was extracted (client side) from the
//! `Cargo.toml` file.

use crate::controllers::frontend_prelude::*;

use crate::models::VersionOwnerAction;
use crate::schema::*;
use crate::views::{EncodableDependency, EncodablePublicUser, EncodableVersion};

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
        .collect();

    #[derive(Serialize)]
    struct R {
        dependencies: Vec<EncodableDependency>,
    }
    Ok(req.json(&R { dependencies: deps }))
}

/// Handles the `GET /crates/:crate_id/:version/authors` route.
pub fn authors(req: &mut dyn RequestExt) -> EndpointResult {
    let (crate_name, semver) = extract_crate_name_and_semver(req)?;
    let conn = req.db_read_only()?;
    let (version, _) = version_and_crate(&conn, crate_name, semver)?;
    let names = version_authors::table
        .filter(version_authors::version_id.eq(version.id))
        .select(version_authors::name)
        .order(version_authors::name)
        .load(&*conn)?;

    // It was imagined that we wold associate authors with users.
    // This was never implemented. This complicated return struct
    // is all that is left, hear for backwards compatibility.
    #[derive(Serialize)]
    struct R {
        users: Vec<EncodablePublicUser>,
        meta: Meta,
    }
    #[derive(Serialize)]
    struct Meta {
        names: Vec<String>,
    }
    Ok(req.json(&R {
        users: vec![],
        meta: Meta { names },
    }))
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

    #[derive(Serialize)]
    struct R {
        version: EncodableVersion,
    }
    Ok(req.json(&R {
        version: EncodableVersion::from(version, &krate.name, published_by, actions),
    }))
}
