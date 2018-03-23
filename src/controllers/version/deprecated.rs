//! Deprecated api endpoints
//!
//! There are no known uses of these endpoints.  There is currently no plan for
//! removing these endpoints.  At a minimum, logs should be reviewed over a
//! period of time to ensure there are no external users of an endpoint before
//! it is removed.

use controllers::prelude::*;

use url;

use views::EncodableVersion;
use models::Version;
use schema::*;

use super::version_and_crate;

/// Handles the `GET /versions` route.
pub fn index(req: &mut Request) -> CargoResult<Response> {
    use diesel::dsl::any;
    let conn = req.db_conn()?;

    // Extract all ids requested.
    let query = url::form_urlencoded::parse(req.query_string().unwrap_or("").as_bytes());
    let ids = query
        .filter_map(|(ref a, ref b)| if *a == "ids[]" { b.parse().ok() } else { None })
        .collect::<Vec<i32>>();

    let versions = versions::table
        .inner_join(crates::table)
        .select((versions::all_columns, crates::name))
        .filter(versions::id.eq(any(ids)))
        .load::<(Version, String)>(&*conn)?
        .into_iter()
        .map(|(version, crate_name)| version.encodable(&crate_name))
        .collect();

    #[derive(Serialize)]
    struct R {
        versions: Vec<EncodableVersion>,
    }
    Ok(req.json(&R { versions: versions }))
}

/// Handles the `GET /versions/:version_id` and
/// `GET /crates/:crate_id/:version` routes.
///
/// The frontend doesn't appear to hit either of these endpoints. Instead the
/// version information appears to be returned by `krate::show`.
///
/// FIXME: These two routes have very different semantics and should be split into
/// a separate function for each endpoint.
pub fn show(req: &mut Request) -> CargoResult<Response> {
    let (version, krate) = match req.params().find("crate_id") {
        Some(..) => version_and_crate(req)?,
        None => {
            let id = &req.params()["version_id"];
            let id = id.parse().unwrap_or(0);
            let conn = req.db_conn()?;
            versions::table
                .find(id)
                .inner_join(crates::table)
                .select((versions::all_columns, ::models::krate::ALL_COLUMNS))
                .first(&*conn)?
        }
    };

    #[derive(Serialize)]
    struct R {
        version: EncodableVersion,
    }
    Ok(req.json(&R {
        version: version.encodable(&krate.name),
    }))
}
