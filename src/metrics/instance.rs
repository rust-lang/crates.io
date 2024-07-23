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

use crate::app::App;
use crate::metrics::macros::metrics;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::AsyncPgConnection;
use prometheus::{
    proto::MetricFamily, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec,
};

metrics! {
    pub struct InstanceMetrics {
        /// Number of idle database connections in the pool
        database_idle_conns: IntGaugeVec["pool"],
        /// Number of used database connections in the pool
        database_used_conns: IntGaugeVec["pool"],
        /// Amount of time required to obtain a database connection
        pub database_time_to_obtain_connection: HistogramVec["pool"],
        /// Number of times the database pool was unavailable and the fallback was used
        pub database_fallback_used: IntGaugeVec["pool"],

        /// Number of requests processed by this instance
        pub requests_total: IntCounter,
        /// Number of requests currently being processed
        pub requests_in_flight: IntGauge,

        /// Response times of our endpoints
        pub response_times: HistogramVec["endpoint"],
        /// Nmber of responses per status code
        pub responses_by_status_code_total: IntCounterVec["status"],
    }

    // All instance metrics will be prefixed with this namespace.
    namespace: "cratesio_instance",
}

impl InstanceMetrics {
    pub fn gather(&self, app: &App) -> prometheus::Result<Vec<MetricFamily>> {
        // Database pool stats
        self.refresh_pool_stats("async_primary", &app.primary_database)?;
        if let Some(follower) = &app.replica_database {
            self.refresh_pool_stats("async_follower", follower)?;
        }

        Ok(self.registry.gather())
    }

    fn refresh_pool_stats(
        &self,
        name: &str,
        pool: &Pool<AsyncPgConnection>,
    ) -> prometheus::Result<()> {
        let status = pool.status();

        self.database_idle_conns
            .get_metric_with_label_values(&[name])?
            .set(status.available as i64);
        self.database_used_conns
            .get_metric_with_label_values(&[name])?
            .set((status.size - status.available) as i64);

        Ok(())
    }
}
