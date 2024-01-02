//! Functionality for downloading crates and maintaining download counts
//!
//! Crate level functionality is located in `krate::downloads`.

use super::version_and_crate;
use crate::controllers::prelude::*;
use crate::db::PoolError;
use crate::middleware::log_request::RequestLogExt;
use crate::models::VersionDownload;
use crate::schema::*;
use crate::util::errors::version_not_found;
use crate::views::EncodableVersionDownload;
use chrono::{Duration, NaiveDate, Utc};
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

    if let Some(version_id) = cache_result {
        app.instance_metrics.version_id_cache_hits.inc();

        // The increment does not happen instantly, but it's deferred to be executed in a batch
        // along with other downloads. See crate::downloads_counter for the implementation.
        app.downloads_counter.increment(version_id);
    } else {
        app.instance_metrics.version_id_cache_misses.inc();

        let version_id = spawn_blocking::<_, _, BoxedAppError>({
            let app = app.clone();
            let crate_name = crate_name.clone();
            let version = version.clone();

            move || {
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
                    let metric = &app.instance_metrics.downloads_select_query_execution_time;
                    let version_id = metric.observe_closure_duration(|| {
                        get_version_id(&crate_name, &version, &mut conn)
                    })?;

                    // The increment does not happen instantly, but it's deferred to be executed in a batch
                    // along with other downloads. See crate::downloads_counter for the implementation.
                    app.downloads_counter.increment(version_id);

                    Ok(Some(version_id))
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

                    Ok(None)
                }
            }
        })
        .await?;

        if let Some(version_id) = version_id {
            let span = info_span!("cache.write", ?cache_key);
            app.version_id_cacher
                .insert(cache_key, version_id)
                .instrument(span)
                .await;
        }
    };

    let redirect_url = app.storage.crate_location(&crate_name, &version);
    if wants_json {
        Ok(Json(json!({ "url": redirect_url })).into_response())
    } else {
        Ok(redirect(redirect_url))
    }
}

#[instrument("db.query", skip(conn), fields(message = "SELECT ... FROM versions"))]
fn get_version_id(krate: &str, version: &str, conn: &mut PgConnection) -> QueryResult<i32> {
    versions::table
        .inner_join(crates::table)
        .select(versions::id)
        .filter(crates::name.eq(&krate))
        .filter(versions::num.eq(&version))
        .first::<i32>(conn)
}

/// Handles the `GET /crates/:crate_id/:version/downloads` route.
pub async fn downloads(
    app: AppState,
    Path((crate_name, version)): Path<(String, String)>,
    req: Parts,
) -> AppResult<Json<Value>> {
    spawn_blocking(move || {
        if semver::Version::parse(&version).is_err() {
            return Err(version_not_found(&crate_name, &version));
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
