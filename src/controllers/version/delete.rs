//! Functionality related to deleting a crate.

use chrono::{Duration, Utc};
use diesel::dsl::count_star;

use crate::controllers::cargo_prelude::*;

use crate::schema::*;
use crate::util::errors::internal;
use crate::views::{EncodableCrate, GoodCrate, PublishWarnings};

use super::version_and_crate;

/// Handles the `DELETE /crates/:crate_id/:version` route.
///
/// Actually deletion is allowed only in the first 24 hours from creation
pub async fn delete(
    app: AppState,
    Path((crate_name, semver)): Path<(String, String)>,
) -> AppResult<Json<GoodCrate>> {
    let conn = app.db_read()?;
    let (version, krate) = version_and_crate(&conn, &crate_name, &semver)?;

    if Utc::now()
        .naive_utc()
        .signed_duration_since(version.created_at)
        > Duration::hours(24)
    {
        return Err(cargo_err(
            "Version deletion is allowed only in the first 24 hours from creation",
        ));
    }

    // Create a transaction on the database, if there are no errors,
    // commit the transactions to delete the version.
    // If there are no remaining versions, delete the crate
    conn.transaction(|| {
        diesel::delete(versions::table.find(version.id)).execute(&*conn)?;

        let top_versions = krate.top_versions(&conn)?;

        // we can't check top_versions to know if there aren't remaining versions
        // because it excludes yanked versions
        let remaining: i64 = versions::table
            .filter(versions::crate_id.eq(krate.id))
            .select(count_star())
            .first(&*conn)
            .optional()?
            .unwrap_or_default();
        if remaining <= 0 {
            diesel::delete(crates::table.find(krate.id)).execute(&*conn)?;

            let uploader = app.config.uploader();
            uploader
                .delete_index(app.http_client(), &krate.name)
                .map_err(|e| internal(format_args!("failed to delete crate: {e}")))?;
        }

        Ok(Json(GoodCrate {
            krate: EncodableCrate::from_minimal(krate, Some(&top_versions), None, true, None),
            warnings: PublishWarnings {
                invalid_categories: vec![],
                invalid_badges: vec![],
                other: vec![],
            },
        }))
    })
}
