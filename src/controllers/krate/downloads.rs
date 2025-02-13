//! Endpoint for exposing crate download counts
//!
//! The endpoint for downloading a crate and exposing version specific
//! download counts are located in `version::downloads`.

use crate::app::AppState;
use crate::controllers::krate::CratePath;
use crate::models::download::Version;
use crate::models::VersionDownload;
use crate::schema::{version_downloads, versions};
use crate::util::errors::AppResult;
use crate::views::EncodableVersionDownload;
use axum_extra::json;
use axum_extra::response::ErasedJson;
use crates_io_diesel_helpers::to_char;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use futures_util::FutureExt;
use std::cmp;

/// Get the download counts for a crate.
///
/// This includes the per-day downloads for the last 90 days and for the
/// latest 5 versions plus the sum of the rest.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/downloads",
    params(CratePath),
    tag = "crates",
    responses((status = 200, description = "Successful Response")),
)]

pub async fn get_crate_downloads(state: AppState, path: CratePath) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;

    use diesel::dsl::*;
    use diesel::sql_types::BigInt;

    let crate_id: i32 = path.load_crate_id(&mut conn).await?;

    let mut versions: Vec<Version> = versions::table
        .filter(versions::crate_id.eq(crate_id))
        .select(Version::as_select())
        .load(&mut conn)
        .await?;

    versions.sort_unstable_by(|a, b| b.num.cmp(&a.num));
    let (latest_five, rest) = versions.split_at(cmp::min(5, versions.len()));

    let sum_downloads = sql::<BigInt>("SUM(version_downloads.downloads)");
    let (downloads, extra) = tokio::try_join!(
        VersionDownload::belonging_to(latest_five)
            .filter(version_downloads::date.gt(date(now - 90.days())))
            .order((
                version_downloads::date.asc(),
                version_downloads::version_id.desc(),
            ))
            .load(&mut conn)
            .boxed(),
        VersionDownload::belonging_to(rest)
            .select((
                to_char(version_downloads::date, "YYYY-MM-DD"),
                sum_downloads,
            ))
            .filter(version_downloads::date.gt(date(now - 90.days())))
            .group_by(version_downloads::date)
            .order(version_downloads::date.asc())
            .load::<ExtraDownload>(&mut conn)
            .boxed(),
    )?;

    let downloads = downloads
        .into_iter()
        .map(VersionDownload::into)
        .collect::<Vec<EncodableVersionDownload>>();

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
