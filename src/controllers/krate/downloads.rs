//! Endpoint for exposing crate download counts
//!
//! The endpoint for downloading a crate and exposing version specific
//! download counts are located in `version::downloads`.

use crate::app::AppState;
use crate::models::{Crate, Version, VersionDownload};
use crate::schema::{crates, version_downloads, versions};
use crate::sql::to_char;
use crate::util::errors::{crate_not_found, AppResult};
use crate::views::EncodableVersionDownload;
use axum::extract::Path;
use axum_extra::json;
use axum_extra::response::ErasedJson;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use std::cmp;

/// Get the download counts for a crate.
///
/// This includes the per-day downloads for the last 90 days and for the
/// latest 5 versions plus the sum of the rest.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/downloads",
    operation_id = "get_crate_downloads",
    tag = "crates",
    responses((status = 200, description = "Successful Response")),
)]

pub async fn downloads(state: AppState, Path(crate_name): Path<String>) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;

    use diesel::dsl::*;
    use diesel::sql_types::BigInt;

    let crate_id: i32 = Crate::by_name(&crate_name)
        .select(crates::id)
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(&crate_name))?;

    let mut versions: Vec<Version> = versions::table
        .filter(versions::crate_id.eq(crate_id))
        .select(Version::as_select())
        .load(&mut conn)
        .await?;

    versions.sort_by_cached_key(|version| cmp::Reverse(semver::Version::parse(&version.num).ok()));
    let (latest_five, rest) = versions.split_at(cmp::min(5, versions.len()));

    let downloads = VersionDownload::belonging_to(latest_five)
        .filter(version_downloads::date.gt(date(now - 90.days())))
        .order((
            version_downloads::date.asc(),
            version_downloads::version_id.desc(),
        ))
        .load(&mut conn)
        .await?
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
        .load(&mut conn)
        .await?;

    #[derive(Serialize, Queryable)]
    struct ExtraDownload {
        date: String,
        downloads: i64,
    }

    Ok(json!({
        "version_downloads": downloads,
        "meta": {
            "extra_downloads": extra,
        },
    }))
}
