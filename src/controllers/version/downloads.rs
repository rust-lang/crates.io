//! Functionality for downloading crates and maintaining download counts
//!
//! Crate level functionality is located in `krate::downloads`.

use controllers::prelude::*;

use chrono::{Duration, NaiveDate, Utc};

use Replica;

use models::{Crate, VersionDownload};
use schema::*;
use views::EncodableVersionDownload;

use super::version_and_crate;

/// Handles the `GET /crates/:crate_id/:version/download` route.
/// This returns a URL to the location where the crate is stored.
pub fn download(req: &mut dyn Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let version = &req.params()["version"];

    // If we are a mirror, ignore failure to update download counts.
    // API-only mirrors won't have any crates in their database, and
    // incrementing the download count will look up the crate in the
    // database. Mirrors just want to pass along a redirect URL.
    if req.app().config.mirror == Replica::ReadOnlyMirror {
        let _ = increment_download_counts(req, crate_name, version);
    } else {
        increment_download_counts(req, crate_name, version)?;
    }

    let redirect_url = req
        .app()
        .config
        .uploader
        .crate_location(crate_name, version)
        .ok_or_else(|| human("crate files not found"))?;

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

fn increment_download_counts(
    req: &dyn Request,
    crate_name: &str,
    version: &str,
) -> CargoResult<()> {
    use self::versions::dsl::*;

    let conn = req.db_conn()?;
    let version_id = versions
        .select(id)
        .filter(crate_id.eq_any(Crate::by_name(crate_name).select(crates::id)))
        .filter(num.eq(version))
        .first(&*conn)?;

    VersionDownload::create_or_increment(version_id, &conn)?;
    Ok(())
}

/// Handles the `GET /crates/:crate_id/:version/downloads` route.
pub fn downloads(req: &mut dyn Request) -> CargoResult<Response> {
    let (version, _) = version_and_crate(req)?;
    let conn = req.db_conn()?;
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
