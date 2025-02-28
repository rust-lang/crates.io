//! Functionality for downloading crates and maintaining download counts
//!
//! Crate level functionality is located in `krate::downloads`.

use super::CrateVersionPath;
use crate::app::AppState;
use crate::models::VersionDownload;
use crate::schema::*;
use crate::util::errors::AppResult;
use crate::util::{RequestUtils, redirect};
use crate::views::EncodableVersionDownload;
use axum::Json;
use axum::extract::{FromRequestParts, Query};
use axum::response::{IntoResponse, Response};
use axum_extra::json;
use chrono::{Duration, NaiveDate, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::request::Parts;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UrlResponse {
    /// The URL to the crate file.
    #[schema(example = "https://static.crates.io/crates/serde/serde-1.0.0.crate")]
    pub url: String,
}

/// Download a crate version.
///
/// This returns a URL to the location where the crate is stored.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/{version}/download",
    params(CrateVersionPath),
    tag = "versions",
    responses(
        (status = 302, description = "Successful Response (default)", headers(("location" = String, description = "The URL to the crate file."))),
        (status = 200, description = "Successful Response (for `content-type: application/json`)", body = inline(UrlResponse)),
    ),
)]
pub async fn download_version(
    app: AppState,
    path: CrateVersionPath,
    req: Parts,
) -> AppResult<Response> {
    let wants_json = req.wants_json();
    let redirect_url = app.storage.crate_location(&path.name, &path.version);
    if wants_json {
        Ok(json!({ "url": redirect_url }).into_response())
    } else {
        Ok(redirect(redirect_url))
    }
}

#[derive(Debug, Deserialize, FromRequestParts, utoipa::IntoParams)]
#[from_request(via(Query))]
#[into_params(parameter_in = Query)]
pub struct DownloadsQueryParams {
    /// Only return download counts before this date.
    #[param(example = "2024-06-28")]
    before_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DownloadsResponse {
    pub version_downloads: Vec<EncodableVersionDownload>,
}

/// Get the download counts for a crate version.
///
/// This includes the per-day downloads for the last 90 days.
#[utoipa::path(
    get,
    path = "/api/v1/crates/{name}/{version}/downloads",
    params(CrateVersionPath, DownloadsQueryParams),
    tag = "versions",
    responses((status = 200, description = "Successful Response", body = inline(DownloadsResponse))),
)]
pub async fn get_version_downloads(
    app: AppState,
    path: CrateVersionPath,
    params: DownloadsQueryParams,
) -> AppResult<Json<DownloadsResponse>> {
    let mut conn = app.db_read().await?;
    let version = path.load_version(&mut conn).await?;

    let cutoff_end_date = params
        .before_date
        .unwrap_or_else(|| Utc::now().date_naive());

    let cutoff_start_date = cutoff_end_date - Duration::days(89);

    let version_downloads = VersionDownload::belonging_to(&version)
        .filter(version_downloads::date.between(cutoff_start_date, cutoff_end_date))
        .order(version_downloads::date)
        .load(&mut conn)
        .await?
        .into_iter()
        .map(VersionDownload::into)
        .collect();

    Ok(Json(DownloadsResponse { version_downloads }))
}
