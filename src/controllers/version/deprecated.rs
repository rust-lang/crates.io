//! Deprecated api endpoints
//!
//! There are no known uses of these endpoints.  There is currently no plan for
//! removing these endpoints.  At a minimum, logs should be reviewed over a
//! period of time to ensure there are no external users of an endpoint before
//! it is removed.

use crate::controllers::prelude::*;

use crate::models::user;
use crate::models::user::UserNoEmailType;
use crate::models::{Crate, Version};
use crate::schema::*;
use crate::views::EncodableVersion;

/// Handles the `GET /versions` route.
pub fn index(req: &mut dyn Request) -> CargoResult<Response> {
    use diesel::dsl::any;
    let conn = req.db_conn()?;

    // Extract all ids requested.
    let query = url::form_urlencoded::parse(req.query_string().unwrap_or("").as_bytes());
    let ids = query
        .filter_map(|(ref a, ref b)| if *a == "ids[]" { b.parse().ok() } else { None })
        .collect::<Vec<i32>>();

    let versions = versions::table
        .inner_join(crates::table)
        .left_outer_join(users::table)
        .select((
            versions::all_columns,
            crates::name,
            user::ALL_COLUMNS.nullable(),
        ))
        .filter(versions::id.eq(any(ids)))
        .load::<(Version, String, Option<UserNoEmailType>)>(&*conn)?
        .into_iter()
        .map(|(version, crate_name, published_by)| {
            version.encodable(&crate_name, published_by.map(From::from))
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
pub fn show_by_id(req: &mut dyn Request) -> CargoResult<Response> {
    let id = &req.params()["version_id"];
    let id = id.parse().unwrap_or(0);
    let conn = req.db_conn()?;
    let (version, krate, published_by): (Version, Crate, Option<UserNoEmailType>) = versions::table
        .find(id)
        .inner_join(crates::table)
        .left_outer_join(users::table)
        .select((
            versions::all_columns,
            crate::models::krate::ALL_COLUMNS,
            user::ALL_COLUMNS.nullable(),
        ))
        .first(&*conn)?;

    #[derive(Serialize)]
    struct R {
        version: EncodableVersion,
    }
    Ok(req.json(&R {
        version: version.encodable(&krate.name, published_by.map(From::from)),
    }))
}
