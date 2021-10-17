//! This module defines all the instance-level metrics of crates.io.
//!
//! Instance-level metrics are collected separately for each instance of the crates.io application,
//! and are then aggregated at the Prometheus level. They're not suited for service-level metrics
//! (like "how many users are there").
//!
//! There are two ways to update instance-level metrics:
//!
//! * Continuously as things happen in the instance: every time something worth recording happens
//!   the application updates the value of the metrics, accessing the metric through
//!   `req.app().instance_metrics.$metric_name`.
//!
//! * When metrics are scraped by Prometheus: every `N` seconds Prometheus sends a request to the
//!   instance asking what the value of the metrics are, and you can update metrics when that
//!   happens by calculating them in the `gather` method.
//!
//! As a rule of thumb, if the metric requires a database query to be updated it's probably a
//! service-level metric, and you should add it to `src/metrics/service.rs` instead.

use crate::util::errors::AppResult;
use crate::{app::App, db::DieselPool};
use prometheus::{
    proto::MetricFamily, Histogram, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec,
};

metrics! {
    pub struct InstanceMetrics {
        /// Number of idle database connections in the pool
        database_idle_conns: IntGaugeVec["pool"],
        /// Number of used database connections in the pool
        database_used_conns: IntGaugeVec["pool"],
        /// Amount of time required to obtain a database connection
        pub database_time_to_obtain_connection: HistogramVec["pool"],

        /// Number of requests processed by this instance
        pub requests_total: IntCounter,
        /// Number of requests currently being processed
        pub requests_in_flight: IntGauge,

        /// Response times of our endpoints
        pub response_times: HistogramVec["endpoint"],
        /// Nmber of responses per status code
        pub responses_by_status_code_total: IntCounterVec["status"],

        /// Number of download requests that were served with an unconditional redirect.
        pub downloads_unconditional_redirects_total: IntCounter,
        /// Number of download requests with a non-canonical crate name.
        pub downloads_non_canonical_crate_name_total: IntCounter,
        /// How long it takes to execute the SELECT query in the download endpoint.
        pub downloads_select_query_execution_time: Histogram,
        /// Number of download requests that are not counted yet.
        downloads_not_counted_total: IntGauge,

        /// Number of version ID cache hits on the download endpoint.
        pub version_id_cache_hits: IntCounter,
        /// Number of version ID cache misses on the download endpoint.
        pub version_id_cache_misses: IntCounter,
    }

    // All instance metrics will be prefixed with this namespace.
    namespace: "cratesio_instance",
}

impl InstanceMetrics {
    pub fn gather(&self, app: &App) -> AppResult<Vec<MetricFamily>> {
        // Database pool stats
        self.refresh_pool_stats("primary", &app.primary_database)?;
        if let Some(follower) = &app.read_only_replica_database {
            self.refresh_pool_stats("follower", follower)?;
        }

        self.downloads_not_counted_total
            .set(app.downloads_counter.pending_count());

        Ok(self.registry.gather())
    }

    fn refresh_pool_stats(&self, name: &str, pool: &DieselPool) -> AppResult<()> {
        let state = pool.state();

        self.database_idle_conns
            .get_metric_with_label_values(&[name])?
            .set(state.idle_connections as i64);
        self.database_used_conns
            .get_metric_with_label_values(&[name])?
            .set((state.connections - state.idle_connections) as i64);

        Ok(())
    }
}
