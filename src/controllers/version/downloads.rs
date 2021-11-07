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

    let mut crate_name = req.params()["crate_id"].clone();
    let version = req.params()["version"].as_str();

    let mut log_metadata = None;

    let cache_key = (crate_name.to_string(), version.to_string());
    if let Some(version_id) = app.version_id_cacher.get(&cache_key) {
        app.instance_metrics.version_id_cache_hits.inc();

        // The increment does not happen instantly, but it's deferred to be executed in a batch
        // along with other downloads. See crate::downloads_counter for the implementation.
        app.downloads_counter.increment(version_id);
    } else {
        app.instance_metrics.version_id_cache_misses.inc();

        // When no database connection is ready unconditional redirects will be performed. This could
        // happen if the pool is not healthy or if an operator manually configured the application to
        // always perform unconditional redirects (for example as part of the mitigations for an
        // outage). See the comments below for a description of what unconditional redirects do.
        let conn = if app.config.force_unconditional_redirects {
            None
        } else {
            match req.db_conn() {
                Ok(conn) => Some(conn),
                Err(PoolError::UnhealthyPool) => None,
                Err(err) => return Err(err.into()),
            }
        };

        if let Some(conn) = &conn {
            use self::versions::dsl::*;

            // Returns the crate name as stored in the database, or an error if we could
            // not load the version ID from the database.
            let (version_id, canonical_crate_name) = app
                .instance_metrics
                .downloads_select_query_execution_time
                .observe_closure_duration(|| {
                    versions
                        .inner_join(crates::table)
                        .select((id, crates::name))
                        .filter(Crate::with_name(&crate_name))
                        .filter(num.eq(version))
                        .first::<(i32, String)>(&**conn)
                })?;

            if canonical_crate_name != crate_name {
                app.instance_metrics
                    .downloads_non_canonical_crate_name_total
                    .inc();
                log_metadata = Some(("bot", "dl"));
                crate_name = canonical_crate_name;
            } else {
                // The version_id is only cached if the provided crate name was canonical.
                // Non-canonical requests fallback to the "slow" path with a DB query, but
                // we typically only get a few hundred non-canonical requests in a day anyway.
                app.version_id_cacher.insert(cache_key, version_id);
            }

            // The increment does not happen instantly, but it's deferred to be executed in a batch
            // along with other downloads. See crate::downloads_counter for the implementation.
            app.downloads_counter.increment(version_id);
        } else {
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
    };

    let redirect_url = req
        .app()
        .config
        .uploader()
        .crate_location(&crate_name, &*version);

    if let Some((key, value)) = log_metadata {
        req.log_metadata(key, value);
    }

    if req.wants_json() {
        Ok(req.json(&json!({ "url": redirect_url })))
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
        .collect::<Vec<EncodableVersionDownload>>();

    Ok(req.json(&json!({ "version_downloads": downloads })))
}
