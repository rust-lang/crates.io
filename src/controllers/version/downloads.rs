//! Functionality for downloading crates and maintaining download counts
//!
//! Crate level functionality is located in `krate::downloads`.

use super::version_and_crate;
use crate::app::AppState;
use crate::models::VersionDownload;
use crate::schema::*;
use crate::util::errors::{version_not_found, AppResult};
use crate::util::{redirect, RequestUtils};
use crate::views::EncodableVersionDownload;
use axum::extract::Path;
use axum::response::{IntoResponse, Response};
use axum_extra::json;
use axum_extra::response::ErasedJson;
use chrono::{Duration, NaiveDate, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;

/// Download a crate version.
///
/// This returns a URL to the location where the crate is stored.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/{version}/download",
    operation_id = "download_version",
    tag = "versions",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn download(
    app: AppState,
    Path((crate_name, version)): Path<(String, String)>,
    req: Parts,
) -> AppResult<Response> {
    let wants_json = req.wants_json();
    let redirect_url = app.storage.crate_location(&crate_name, &version);
    if wants_json {
        Ok(json!({ "url": redirect_url }).into_response())
    } else {
        Ok(redirect(redirect_url))
    }
}

/// Get the download counts for a crate version.
///
/// This includes the per-day downloads for the last 90 days.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/{version}/downloads",
    operation_id = "get_version_downloads",
    tag = "versions",
    responses((status = 200, description = "Successful Response")),
)]
pub async fn downloads(
    app: AppState,
    Path((crate_name, version)): Path<(String, String)>,
    req: Parts,
) -> AppResult<ErasedJson> {
    if semver::Version::parse(&version).is_err() {
        return Err(version_not_found(&crate_name, &version));
    }

    let mut conn = app.db_read().await?;
    let (version, _) = version_and_crate(&mut conn, &crate_name, &version).await?;

    let cutoff_end_date = req
        .query()
        .get("before_date")
        .and_then(|d| NaiveDate::parse_from_str(d, "%F").ok())
        .unwrap_or_else(|| Utc::now().date_naive());
    let cutoff_start_date = cutoff_end_date - Duration::days(89);

    let downloads = VersionDownload::belonging_to(&version)
        .filter(version_downloads::date.between(cutoff_start_date, cutoff_end_date))
        .order(version_downloads::date)
        .load(&mut conn)
        .await?
        .into_iter()
        .map(VersionDownload::into)
        .collect::<Vec<EncodableVersionDownload>>();

    Ok(json!({ "version_downloads": downloads }))
}
