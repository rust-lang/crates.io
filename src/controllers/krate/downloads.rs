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
use axum::Json;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::Value;

/// Handles the `GET /crates/:crate_id/downloads` route.
pub async fn downloads(state: AppState, Path(crate_name): Path<String>) -> AppResult<Json<Value>> {
    let mut conn = state.db_read().await?;

    use diesel::dsl::*;
    use diesel::sql_types::BigInt;

    let crate_id: i32 = Crate::by_name(&crate_name)
        .select(crates::id)
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(&crate_name))?;

    let versions: Vec<Version> = versions::table
        .filter(versions::crate_id.eq(crate_id))
        .load(&mut conn)
        .await?;

    let top_downloaded_versions: Vec<(i32,)> = VersionDownload::belonging_to(&versions)
        .group_by(version_downloads::version_id)
        .select((version_downloads::version_id,))
        .order(sum(version_downloads::downloads).desc())
        .limit(5)
        .load(&mut conn)
        .await?;
    let (top_five, rest): (Vec<_>, Vec<_>) = versions
        .iter()
        .partition(|v| top_downloaded_versions.contains(&(v.id,)));

    let downloads = VersionDownload::belonging_to(&top_five)
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
    let extra: Vec<ExtraDownload> = VersionDownload::belonging_to(&rest)
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

    Ok(Json(json!({
        "version_downloads": downloads,
        "meta": {
            "extra_downloads": extra,
        },
    })))
}
