//! Functionality for downloading crates and maintaining download counts
//!
//! Crate level functionality is located in `krate::downloads`.

use crate::controllers::prelude::*;

use chrono::{Duration, NaiveDate, Utc};

use crate::models::{Crate, VersionDownload};
use crate::schema::*;
use crate::views::EncodableVersionDownload;

use super::{extract_crate_name_and_semver, version_and_crate};

/// Handles the `GET /crates/:crate_id/:version/download` route.
/// This returns a URL to the location where the crate is stored.
pub fn download(req: &mut dyn RequestExt) -> EndpointResult {
    let recorder = req.timing_recorder();

    let crate_name = &req.params()["crate_id"];
    let version = &req.params()["version"];

    let (version_id, canonical_crate_name): (_, String) = {
        use self::versions::dsl::*;

        let conn = recorder.record("get_conn", || req.db_conn())?;

        // Returns the crate name as stored in the database, or an error if we could
        // not load the version ID from the database.
        recorder.record("get_version", || {
            versions
                .inner_join(crates::table)
                .select((id, crates::name))
                .filter(Crate::with_name(crate_name))
                .filter(num.eq(version))
                .first(&*conn)
        })?
    };

    // The increment does not happen instantly, but it's deferred to be executed in a batch
    // along with other downloads. See crate::downloads_counter for the implementation.
    req.app().downloads_counter.increment(version_id);

    let redirect_url = req
        .app()
        .config
        .uploader
        .crate_location(&canonical_crate_name, version);

    if &canonical_crate_name != crate_name {
        req.log_metadata("bot", "dl");
    }

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

/// Handles the `GET /crates/:crate_id/:version/downloads` route.
pub fn downloads(req: &mut dyn RequestExt) -> EndpointResult {
    let (crate_name, semver) = extract_crate_name_and_semver(req)?;

    let conn = req.db_read_only()?;
    let (version, _) = version_and_crate(&conn, crate_name, semver)?;

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
        .map(VersionDownload::into)
        .collect();

    #[derive(Serialize)]
    struct R {
        version_downloads: Vec<EncodableVersionDownload>,
    }
    Ok(req.json(&R {
        version_downloads: downloads,
    }))
}
