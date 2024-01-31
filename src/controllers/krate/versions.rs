//! Endpoint for versions of a crate

use std::cmp::Reverse;

use crate::controllers::frontend_prelude::*;

use crate::models::{Crate, CrateVersions, User, Version, VersionOwnerAction};
use crate::schema::{users, versions};
use crate::util::errors::crate_not_found;
use crate::views::EncodableVersion;

/// Handles the `GET /crates/:crate_id/versions` route.
// FIXME: Not sure why this is necessary since /crates/:crate_id returns
// this information already, but ember is definitely requesting it
pub async fn versions(state: AppState, Path(crate_name): Path<String>) -> AppResult<Json<Value>> {
    spawn_blocking(move || {
        let conn = &mut *state.db_read()?;

        let krate: Crate = Crate::by_name(&crate_name)
            .first(conn)
            .optional()?
            .ok_or_else(|| crate_not_found(&crate_name))?;

        let mut versions_and_publishers: Vec<(Version, Option<User>)> = krate
            .all_versions()
            .left_outer_join(users::table)
            .select((versions::all_columns, users::all_columns.nullable()))
            .load(conn)?;

        versions_and_publishers
            .sort_by_cached_key(|(version, _)| Reverse(semver::Version::parse(&version.num).ok()));

        let versions = versions_and_publishers
            .iter()
            .map(|(v, _)| v)
            .cloned()
            .collect::<Vec<_>>();
        let versions = versions_and_publishers
            .into_iter()
            .zip(VersionOwnerAction::for_versions(conn, &versions)?)
            .map(|((v, pb), aas)| EncodableVersion::from(v, &crate_name, pb, aas))
            .collect::<Vec<_>>();

        Ok(Json(json!({ "versions": versions })))
    })
    .await
}
