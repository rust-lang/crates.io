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

use crate::metrics::macros::metrics;
use crate::schema::{background_jobs, crates, versions};
use crate::util::errors::AppResult;
use diesel::{dsl::count_star, prelude::*};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use prometheus::core::Collector;
use prometheus::proto::{Metric, MetricFamily};
use prometheus::{IntGauge, IntGaugeVec};
use std::collections::HashMap;

metrics! {
    pub struct ServiceMetrics {
        /// Number of crates ever published
        crates_total: IntGauge,
        /// Number of versions ever published
        versions_total: IntGauge,
        /// Number of queued up background jobs
        background_jobs: IntGaugeVec["priority", "job"],
    }

    // All service metrics will be prefixed with this namespace.
    namespace: "cratesio_service",
}

impl ServiceMetrics {
    pub(crate) async fn gather(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> AppResult<Vec<MetricFamily>> {
        self.crates_total
            .set(crates::table.select(count_star()).first(conn).await?);
        self.versions_total
            .set(versions::table.select(count_star()).first(conn).await?);

        let queued_jobs = background_jobs::table
            .group_by((background_jobs::job_type, background_jobs::priority))
            .select((
                background_jobs::job_type,
                background_jobs::priority,
                count_star(),
            ))
            .load::<(String, i16, i64)>(conn)
            .await?;

        let mut counts: HashMap<(String, String), i64> = queued_jobs
            .into_iter()
            .map(|(job, priority, count)| ((priority.to_string(), job), count))
            .collect();

        for family in self.background_jobs.collect() {
            for metric in family.get_metric() {
                let priority = label_value(metric, "priority");
                let job = label_value(metric, "job");
                counts.entry((priority, job)).or_insert(0);
            }
        }

        for ((priority, job), count) in counts {
            self.background_jobs
                .get_metric_with_label_values(&[&priority, &job])?
                .set(count);
        }

        Ok(self.registry.gather())
    }
}

/// Reads the value of a named label from a gathered metric, returning an empty
/// string when the label is absent.
fn label_value(metric: &Metric, name: &str) -> String {
    metric
        .get_label()
        .iter()
        .find(|label| label.name() == name)
        .map(|label| label.value().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_some_eq;
    use crates_io_test_db::TestDatabase;

    async fn enqueue(conn: &mut AsyncPgConnection, job: &str, priority: i16) -> AppResult<()> {
        diesel::insert_into(background_jobs::table)
            .values((
                background_jobs::job_type.eq(job),
                background_jobs::data.eq(serde_json::json!({})),
                background_jobs::priority.eq(priority),
            ))
            .execute(conn)
            .await?;

        Ok(())
    }

    /// Collects the `background_jobs` gauge into a `(priority, job) -> value` map.
    fn job_counts(families: &[MetricFamily]) -> HashMap<(String, String), i64> {
        families
            .iter()
            .find(|family| family.name() == "cratesio_service_background_jobs")
            .into_iter()
            .flat_map(|family| family.get_metric())
            .map(|metric| {
                let key = (label_value(metric, "priority"), label_value(metric, "job"));
                (key, metric.get_gauge().get_value() as i64)
            })
            .collect()
    }

    #[tokio::test]
    async fn test_drained_background_jobs_report_zero() -> AppResult<()> {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        enqueue(&mut conn, "job_a", 0).await?;
        enqueue(&mut conn, "job_b", 10).await?;

        let metrics = ServiceMetrics::new()?;
        let counts = job_counts(&metrics.gather(&mut conn).await?);
        assert_some_eq!(counts.get(&("0".into(), "job_a".into())), &1);
        assert_some_eq!(counts.get(&("10".into(), "job_b".into())), &1);

        // Drain `job_a`, leaving `job_b` queued.
        diesel::delete(background_jobs::table.filter(background_jobs::job_type.eq("job_a")))
            .execute(&mut conn)
            .await?;

        // `job_a` must still be reported, now at zero, instead of disappearing.
        let counts = job_counts(&metrics.gather(&mut conn).await?);
        assert_some_eq!(counts.get(&("0".into(), "job_a".into())), &0);
        assert_some_eq!(counts.get(&("10".into(), "job_b".into())), &1);

        Ok(())
    }
}
