//! Deprecated api endpoints
//!
//! There are no known uses of these endpoints.  There is currently no plan for
//! removing these endpoints.  At a minimum, logs should be reviewed over a
//! period of time to ensure there are no external users of an endpoint before
//! it is removed.

use crate::controllers::frontend_prelude::*;

use crate::models::{Crate, User, Version, VersionOwnerAction};
use crate::schema::*;
use crate::views::EncodableVersion;

/// Handles the `GET /versions` route.
pub fn index(req: &mut dyn Request) -> AppResult<Response> {
    use diesel::dsl::any;
    let conn = req.db_conn()?;

    // Extract all ids requested.
    let query = url::form_urlencoded::parse(req.query_string().unwrap_or("").as_bytes());
    let ids = query
        .filter_map(|(ref a, ref b)| if *a == "ids[]" { b.parse().ok() } else { None })
        .collect::<Vec<i32>>();

    let versions_and_publishers = versions::table
        .inner_join(crates::table)
        .left_outer_join(users::table)
        .select((
            versions::all_columns,
            crates::name,
            users::all_columns.nullable(),
        ))
        .filter(versions::id.eq(any(ids)))
        .load::<(Version, String, Option<User>)>(&*conn)?;
    let versions = versions_and_publishers
        .iter()
        .map(|(v, _, _)| v)
        .cloned()
        .collect::<Vec<_>>();
    let versions = versions_and_publishers
        .into_iter()
        .zip(VersionOwnerAction::for_versions(&conn, &versions)?.into_iter())
        .map(|((version, crate_name, published_by), actions)| {
            version.encodable(&crate_name, published_by, actions)
        })
        .collect();

    #[derive(Serialize)]
    struct R {
        versions: Vec<EncodableVersion>,
    }
    Ok(req.json(&R { versions }))
}

/// Handles the `GET /versions/:version_id` route.
/// The frontend doesn't appear to hit this endpoint. Instead, the version information appears to
/// be returned by `krate::show`.
pub fn show_by_id(req: &mut dyn Request) -> AppResult<Response> {
    let id = &req.params()["version_id"];
    let id = id.parse().unwrap_or(0);
    let conn = req.db_conn()?;
    let (version, krate, published_by): (Version, Crate, Option<User>) = versions::table
        .find(id)
        .inner_join(crates::table)
        .left_outer_join(users::table)
        .select((
            versions::all_columns,
            crate::models::krate::ALL_COLUMNS,
            users::all_columns.nullable(),
        ))
        .first(&*conn)?;
    let audit_actions = VersionOwnerAction::by_version(&conn, &version)?;

    #[derive(Serialize)]
    struct R {
        version: EncodableVersion,
    }
    Ok(req.json(&R {
        version: version.encodable(&krate.name, published_by, audit_actions),
    }))
}
