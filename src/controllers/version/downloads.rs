//! Functionality for downloading crates and maintaining download counts
//!
//! Crate level functionality is located in `krate::downloads`.

use super::version_and_crate;
use crate::controllers::prelude::*;
use crate::db::PoolError;
use crate::middleware::log_request::RequestLogExt;
use crate::models::{Crate, VersionDownload};
use crate::schema::*;
use crate::views::EncodableVersionDownload;
use chrono::{Duration, NaiveDate, Utc};
use std::fmt::Display;
use tokio::runtime::Handle;
use tracing::Instrument;

/// Handles the `GET /crates/:crate_id/:version/download` route.
/// This returns a URL to the location where the crate is stored.
pub async fn download(
    app: AppState,
    Path((crate_name, version)): Path<(String, String)>,
    req: Parts,
) -> AppResult<Response> {
    let wants_json = req.wants_json();

    let cache_key = (crate_name.to_string(), version.to_string());

    let cache_result = app
        .version_id_cacher
        .get(&cache_key)
        .instrument(info_span!("cache.read", ?cache_key))
        .await;

    let (crate_name, version) = if let Some(version_id) = cache_result {
        app.instance_metrics.version_id_cache_hits.inc();

        // The increment does not happen instantly, but it's deferred to be executed in a batch
        // along with other downloads. See crate::downloads_counter for the implementation.
        app.downloads_counter.increment(version_id);

        (crate_name, version)
    } else {
        app.instance_metrics.version_id_cache_misses.inc();

        let app = app.clone();
        conduit_compat(move || {
            // When no database connection is ready unconditional redirects will be performed. This could
            // happen if the pool is not healthy or if an operator manually configured the application to
            // always perform unconditional redirects (for example as part of the mitigations for an
            // outage). See the comments below for a description of what unconditional redirects do.
            let conn = if app.config.force_unconditional_redirects {
                None
            } else {
                match app.db_read_prefer_primary() {
                    Ok(conn) => Some(conn),
                    Err(PoolError::UnhealthyPool) => None,
                    Err(err) => return Err(err.into()),
                }
            };

            if let Some(mut conn) = conn {
                // Returns the crate name as stored in the database, or an error if we could
                // not load the version ID from the database.
                let (version_id, canonical_crate_name) = app
                    .instance_metrics
                    .downloads_select_query_execution_time
                    .observe_closure_duration(|| {
                        info_span!("db.query", message = "SELECT ... FROM versions").in_scope(
                            || {
                                versions::table
                                    .inner_join(crates::table)
                                    .select((versions::id, crates::name))
                                    .filter(Crate::with_name(&crate_name))
                                    .filter(versions::num.eq(&version))
                                    .first::<(i32, String)>(&mut *conn)
                            },
                        )
                    })?;

                // The increment does not happen instantly, but it's deferred to be executed in a batch
                // along with other downloads. See crate::downloads_counter for the implementation.
                app.downloads_counter.increment(version_id);

                if canonical_crate_name != crate_name {
                    app.instance_metrics
                        .downloads_non_canonical_crate_name_total
                        .inc();
                    req.request_log().add("bot", "dl");

                    if app.config.reject_non_canonical_downloads {
                        return Err(Box::new(NonCanonicalDownload {
                            requested_name: crate_name,
                            canonical_name: canonical_crate_name,
                        }));
                    }
                } else {
                    // The version_id is only cached if the provided crate name was canonical.
                    // Non-canonical requests fallback to the "slow" path with a DB query, but
                    // we typically only get a few hundred non-canonical requests in a day anyway.
                    let span = info_span!("cache.write", ?cache_key);

                    // SAFETY: This block_on should not panic. block_on will panic if the
                    // current thread is an executor thread of a Tokio runtime. (Will panic
                    // by "Cannot start a runtime from within a runtime"). Here, we are in
                    // a spawn_blocking call because of conduit_compat, so our current thread
                    // is not an executor of the runtime.
                    Handle::current().block_on(
                        app.version_id_cacher
                            .insert(cache_key, version_id)
                            .instrument(span),
                    );
                }

                Ok((canonical_crate_name, version))
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
                // checking whether the crate exists or the right name is used. Non-Cargo clients might
                // get a 404 response instead of a 500, but that's worth it.
                //
                // Without a working database we also can't count downloads, but that's also less
                // critical than keeping Cargo downloads operational.

                app.instance_metrics
                    .downloads_unconditional_redirects_total
                    .inc();

                req.request_log().add("unconditional_redirect", "true");

                Ok((crate_name, version))
            }
        })
        .await?
    };

    let redirect_url = app.storage.crate_location(&crate_name, &version);
    if wants_json {
        Ok(Json(json!({ "url": redirect_url })).into_response())
    } else {
        Ok(redirect(redirect_url))
    }
}

#[derive(Debug)]
struct NonCanonicalDownload {
    requested_name: String,
    canonical_name: String,
}

impl Display for NonCanonicalDownload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Your request is for a version of the `{requested_name}` crate, \
            but that crate is actually named `{canonical_name}`. Support for \
            \"non-canonical\" downloads has been deprecated and disabled. See \
            https://blog.rust-lang.org/2023/10/27/crates-io-non-canonical-downloads.html \
            for more detail.",
            requested_name = self.requested_name,
            canonical_name = self.canonical_name,
        )
    }
}

impl AppError for NonCanonicalDownload {
    fn response(&self) -> Response {
        (StatusCode::NOT_FOUND, self.to_string()).into_response()
    }
}

/// Handles the `GET /crates/:crate_id/:version/downloads` route.
pub async fn downloads(
    app: AppState,
    Path((crate_name, version)): Path<(String, String)>,
    req: Parts,
) -> AppResult<Json<Value>> {
    conduit_compat(move || {
        if semver::Version::parse(&version).is_err() {
            return Err(cargo_err(&format_args!("invalid semver: {version}")));
        }

        let conn = &mut *app.db_read()?;
        let (version, _) = version_and_crate(conn, &crate_name, &version)?;

        let cutoff_end_date = req
            .query()
            .get("before_date")
            .and_then(|d| NaiveDate::parse_from_str(d, "%F").ok())
            .unwrap_or_else(|| Utc::now().date_naive());
        let cutoff_start_date = cutoff_end_date - Duration::days(89);

        let downloads = VersionDownload::belonging_to(&version)
            .filter(version_downloads::date.between(cutoff_start_date, cutoff_end_date))
            .order(version_downloads::date)
            .load(conn)?
            .into_iter()
            .map(VersionDownload::into)
            .collect::<Vec<EncodableVersionDownload>>();

        Ok(Json(json!({ "version_downloads": downloads })))
    })
    .await
}
