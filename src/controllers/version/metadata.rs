//! Endpoints that expose metadata about crate versions
//!
//! These endpoints provide data that could be obtained direclty from the
//! index or cached metadata which was extracted (client side) from the
//! `Cargo.toml` file.

use controllers::prelude::*;

use schema::*;
use views::{EncodableDependency, EncodablePublicUser};

use super::version_and_crate;

/// Handles the `GET /crates/:crate_id/:version/dependencies` route.
///
/// This information can be obtained direclty from the index.
///
/// In addition to returning cached data from the index, this returns
/// fields for `id`, `version_id`, and `downloads` (which appears to always
/// be 0)
pub fn dependencies(req: &mut dyn Request) -> CargoResult<Response> {
    let (version, _) = version_and_crate(req)?;
    let conn = req.db_conn()?;
    let deps = version.dependencies(&*conn)?;
    let deps = deps.into_iter()
        .map(|(dep, crate_name)| dep.encodable(&crate_name, None))
        .collect();

    #[derive(Serialize)]
    struct R {
        dependencies: Vec<EncodableDependency>,
    }
    Ok(req.json(&R { dependencies: deps }))
}

/// Handles the `GET /crates/:crate_id/:version/authors` route.
pub fn authors(req: &mut dyn Request) -> CargoResult<Response> {
    let (version, _) = version_and_crate(req)?;
    let conn = req.db_conn()?;
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
