//! Functionality for downloading crates and maintaining download counts
//!
//! Crate level functionality is located in `krate::downloads`.

use crate::controllers::prelude::*;

use chrono::{Duration, NaiveDate, Utc};

use crate::models::{Crate, VersionDownload};
use crate::schema::*;
use crate::views::EncodableVersionDownload;

use super::{extract_crate_name, extract_semver};

/// Handles the `GET /crates/:crate_id/:version/download` route.
/// This returns a URL to the location where the crate is stored.
pub fn download(req: &mut dyn Request) -> AppResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let version = &req.params()["version"];

    let crate_name = increment_download_counts(req, crate_name, version)?;

    let redirect_url = req
        .app()
        .config
        .uploader
        .crate_location(&crate_name, version);

    if req.wants_json() {
        #[derive(Serialize)]
        struct R {
            url: String,
        }
        Ok(req.json(&R { url: redirect_url }))
    } else {
        Ok(req.redirect(redirect_url))
    }
}

/// Increment the download counts for a given crate version.
///
/// Returns the crate name as stored in the database, or an error if we could
/// not load the version ID from the database.
///
/// This ignores any errors that occur updating the download count. Failure is
/// expected if the application is in read only mode, or for API-only mirrors.
/// Even if failure occurs for unexpected reasons, we would rather have `cargo
/// build` succeed and not count the download than break people's builds.
fn increment_download_counts(
    req: &dyn Request,
    crate_name: &str,
    version: &str,
) -> AppResult<String> {
    use self::versions::dsl::*;

    let conn = req.db_conn()?;
    let (version_id, crate_name) = versions
        .inner_join(crates::table)
        .select((id, crates::name))
        .filter(Crate::with_name(crate_name))
        .filter(num.eq(version))
        .first(&*conn)?;

    // Wrap in a transaction so we don't poison the outer transaction if this
    // fails
    let _ = conn.transaction(|| VersionDownload::create_or_increment(version_id, &conn));
    Ok(crate_name)
}

/// Handles the `GET /crates/:crate_id/:version/downloads` route.
pub fn downloads(req: &mut dyn Request) -> AppResult<Response> {
    let crate_name = extract_crate_name(req);
    let semver = extract_semver(req)?;

    let conn = req.db_read_only()?;
    let version = Crate::by_name(crate_name)
        .first::<Crate>(&*conn)?
        .find_version(&conn, semver)?;

    let cutoff_end_date = req
        .query()
        .get("before_date")
        .and_then(|d| NaiveDate::parse_from_str(d, "%F").ok())
        .unwrap_or_else(|| Utc::today().naive_utc());
    let cutoff_start_date = cutoff_end_date - Duration::days(89);

    let downloads = VersionDownload::belonging_to(&version)
        .filter(version_downloads::date.between(cutoff_start_date, cutoff_end_date))
        .order(version_downloads::date)
        .load(&*conn)?
        .into_iter()
        .map(VersionDownload::encodable)
        .collect();

    #[derive(Serialize)]
    struct R {
        version_downloads: Vec<EncodableVersionDownload>,
    }
    Ok(req.json(&R {
        version_downloads: downloads,
    }))
}
