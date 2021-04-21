//! Functionality for downloading crates and maintaining download counts
//!
//! Crate level functionality is located in `krate::downloads`.

use super::{extract_crate_name_and_semver, version_and_crate};
use crate::controllers::prelude::*;
use crate::db::PoolError;
use crate::models::{Crate, VersionDownload};
use crate::schema::*;
use crate::views::EncodableVersionDownload;
use chrono::{Duration, NaiveDate, Utc};

/// Handles the `GET /crates/:crate_id/:version/download` route.
/// This returns a URL to the location where the crate is stored.
pub fn download(req: &mut dyn RequestExt) -> EndpointResult {
    let app = req.app().clone();
    let recorder = req.timing_recorder();

    let mut crate_name = req.params()["crate_id"].clone();
    let version = req.params()["version"].as_str();

    let mut log_metadata = None;
    match recorder.record("get_conn", || req.db_conn()) {
        Ok(conn) => {
            use self::versions::dsl::*;

            // Returns the crate name as stored in the database, or an error if we could
            // not load the version ID from the database.
            let (version_id, canonical_crate_name) = recorder.record("get_version", || {
                versions
                    .inner_join(crates::table)
                    .select((id, crates::name))
                    .filter(Crate::with_name(&crate_name))
                    .filter(num.eq(version))
                    .first::<(i32, String)>(&*conn)
            })?;

            if canonical_crate_name != crate_name {
                app.instance_metrics
                    .downloads_non_canonical_crate_name_total
                    .inc();
                log_metadata = Some(("bot", "dl"));
            }
            crate_name = canonical_crate_name;

            // The increment does not happen instantly, but it's deferred to be executed in a batch
            // along with other downloads. See crate::downloads_counter for the implementation.
            app.downloads_counter.increment(version_id);
        }
        Err(PoolError::UnhealthyPool) => {
            // The download endpoint is the most critical route in the whole crates.io application,
            // as it's relied upon by users and automations to download crates. Keeping it working
            // is the most important thing for us.
            //
            // The endpoint relies on the database to fetch the canonical crate name (with the
            // right capitalization and hyphenation), but that's only needed to serve clients who
            // don't call the endpoint with the crate's canonical name.
            //
            // Thankfully Cargo always uses the right name when calling the endpoint, and we can
            // keep it working during a full database outage by unconditionally redirecting without
            // checking whether the crate exists or the rigth name is used. Non-Cargo clients might
            // get a 404 response instead of a 500, but that's worth it.
            //
            // Without a working database we also can't count downloads, but that's also less
            // critical than keeping Cargo downloads operational.

            app.instance_metrics
                .downloads_unconditional_redirects_total
                .inc();
            log_metadata = Some(("unconditional_redirect", "true"));
        }
        Err(err) => return Err(err.into()),
    }

    let redirect_url = req
        .app()
        .config
        .uploader
        .crate_location(&crate_name, &*version);

    if let Some((key, value)) = log_metadata {
        req.log_metadata(key, value);
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
