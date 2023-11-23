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
pub async fn index(app: AppState, req: Parts) -> AppResult<Json<Value>> {
    spawn_blocking(move || {
        let conn = &mut *app.db_read()?;

        // Extract all ids requested.
        let query = url::form_urlencoded::parse(req.uri.query().unwrap_or("").as_bytes());
        let ids = query
            .filter_map(|(ref a, ref b)| if *a == "ids[]" { b.parse().ok() } else { None })
            .collect::<Vec<i32>>();

        let versions_and_publishers: Vec<(Version, String, Option<User>)> = versions::table
            .inner_join(crates::table)
            .left_outer_join(users::table)
            .select((
                versions::all_columns,
                crates::name,
                users::all_columns.nullable(),
            ))
            .filter(versions::id.eq_any(ids))
            .load(conn)?;
        let versions = versions_and_publishers
            .iter()
            .map(|(v, _, _)| v)
            .cloned()
            .collect::<Vec<_>>();
        let versions = versions_and_publishers
            .into_iter()
            .zip(VersionOwnerAction::for_versions(conn, &versions)?)
            .map(|((version, crate_name, published_by), actions)| {
                EncodableVersion::from(version, &crate_name, published_by, actions)
            })
            .collect::<Vec<_>>();

        Ok(Json(json!({ "versions": versions })))
    })
    .await
}

/// Handles the `GET /versions/:version_id` route.
/// The frontend doesn't appear to hit this endpoint. Instead, the version information appears to
/// be returned by `krate::show`.
pub async fn show_by_id(state: AppState, Path(id): Path<i32>) -> AppResult<Json<Value>> {
    spawn_blocking(move || {
        let conn = &mut *state.db_read()?;
        let (version, krate, published_by): (Version, Crate, Option<User>) = versions::table
            .find(id)
            .inner_join(crates::table)
            .left_outer_join(users::table)
            .select((
                versions::all_columns,
                Crate::as_select(),
                users::all_columns.nullable(),
            ))
            .first(conn)?;
        let audit_actions = VersionOwnerAction::by_version(conn, &version)?;

        let version = EncodableVersion::from(version, &krate.name, published_by, audit_actions);
        Ok(Json(json!({ "version": version })))
    })
    .await
}
