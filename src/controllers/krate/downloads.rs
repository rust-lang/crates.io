//! Endpoint for exposing crate download counts
//!
//! The endpoint for downloading a crate and exposing version specific
//! download counts are located in `version::downloads`.

use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use std::cmp;

use crate::controllers::frontend_prelude::*;

use crate::models::{Crate, Version, VersionDownload};
use crate::schema::{crates, version_downloads, versions};
use crate::sql::to_char;
use crate::util::errors::crate_not_found;
use crate::views::EncodableVersionDownload;

/// Handles the `GET /crates/:crate_id/downloads` route.
pub async fn downloads(state: AppState, Path(crate_name): Path<String>) -> AppResult<Json<Value>> {
    let conn = state.db_read().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        use diesel::dsl::*;
        use diesel::sql_types::BigInt;

        let crate_id: i32 = Crate::by_name(&crate_name)
            .select(crates::id)
            .first(conn)
            .optional()?
            .ok_or_else(|| crate_not_found(&crate_name))?;

        let mut versions: Vec<Version> = versions::table
            .filter(versions::crate_id.eq(crate_id))
            .load(conn)?;
        versions
            .sort_by_cached_key(|version| cmp::Reverse(semver::Version::parse(&version.num).ok()));
        let (latest_five, rest) = versions.split_at(cmp::min(5, versions.len()));

        let downloads = VersionDownload::belonging_to(latest_five)
            .filter(version_downloads::date.gt(date(now - 90.days())))
            .order((
                version_downloads::date.asc(),
                version_downloads::version_id.desc(),
            ))
            .load(conn)?
            .into_iter()
            .map(VersionDownload::into)
            .collect::<Vec<EncodableVersionDownload>>();

        let sum_downloads = sql::<BigInt>("SUM(version_downloads.downloads)");
        let extra: Vec<ExtraDownload> = VersionDownload::belonging_to(rest)
            .select((
                to_char(version_downloads::date, "YYYY-MM-DD"),
                sum_downloads,
            ))
            .filter(version_downloads::date.gt(date(now - 90.days())))
            .group_by(version_downloads::date)
            .order(version_downloads::date.asc())
            .load(conn)?;

        #[derive(Serialize, Queryable)]
        struct ExtraDownload {
            date: String,
            downloads: i64,
        }

        Ok(Json(json!({
            "version_downloads": downloads,
            "meta": {
                "extra_downloads": extra,
            },
        })))
    })
    .await
}
