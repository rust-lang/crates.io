//! This module defines all the service-level metrics of crates.io.
//!
//! Service-level metrics are collected for the whole service, without querying the individual
//! instances of the application. They're not suited for instance-level metrics (like "how many
//! requests were processed" or "how many connections are left in the database pool").
//!
//! Service-level metrics should **never** be updated around the codebase: instead all the updates
//! should happen inside the `gather` method. A database connection is available inside the method.
//!
//! As a rule of thumb, if the metric is not straight up fetched from the database it's probably an
//! instance-level metric, and you should add it to `src/metrics/instance.rs`.

use crate::schema::{background_jobs, crates, versions};
use crate::util::errors::AppResult;
use diesel::{dsl::count_star, prelude::*, PgConnection};
use prometheus::{proto::MetricFamily, IntGauge};

metrics! {
    pub struct ServiceMetrics {
        /// Number of crates ever published
        crates_total: IntGauge,
        /// Number of versions ever published
        versions_total: IntGauge,
        /// Number of queued up background jobs
        background_jobs: IntGauge,
    }

    // All service metrics will be prefixed with this namespace.
    namespace: "cratesio_service",
}

impl ServiceMetrics {
    pub(crate) fn gather(&self, conn: &PgConnection) -> AppResult<Vec<MetricFamily>> {
        self.crates_total
            .set(crates::table.select(count_star()).first(conn)?);
        self.versions_total
            .set(versions::table.select(count_star()).first(conn)?);
        self.background_jobs
            .set(background_jobs::table.select(count_star()).first(conn)?);

        Ok(self.registry.gather())
    }
}
