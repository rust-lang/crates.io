use super::SharedMetrics;
use super::consts::{
    BACKGROUND_JOBS, BACKGROUND_JOBS_OLDEST_AGE, CRATES_TOTAL, DB_DUMP_SIZE_BYTES,
    DB_DUMP_UPLOAD_DURATION_NS, FORMAT, JOB, PRIORITY, VERSIONS_TOTAL,
};
use super::otel::kv;
use derive_more::Deref;
use opentelemetry::metrics::{Gauge, Meter};
use std::time::Duration;

/// OpenTelemetry instruments recorded by the background worker.
#[derive(Clone, Debug, Deref)]
pub struct WorkerMetrics {
    #[deref]
    shared: SharedMetrics,

    background_jobs: Gauge<i64>,
    background_jobs_oldest_age: Gauge<f64>,
    crates_total: Gauge<i64>,
    versions_total: Gauge<i64>,
    db_dump_size: Gauge<u64>,
    db_dump_upload_duration: Gauge<u64>,
}

impl WorkerMetrics {
    /// Creates the worker instruments from `meter`.
    pub fn new(meter: &Meter) -> Self {
        let shared = SharedMetrics::new(meter);
        let background_jobs = meter.i64_gauge(BACKGROUND_JOBS).build();
        let background_jobs_oldest_age = meter
            .f64_gauge(BACKGROUND_JOBS_OLDEST_AGE)
            .with_unit("s")
            .build();
        let crates_total = meter.i64_gauge(CRATES_TOTAL).build();
        let versions_total = meter.i64_gauge(VERSIONS_TOTAL).build();
        let db_dump_size = meter.u64_gauge(DB_DUMP_SIZE_BYTES).build();
        let db_dump_upload_duration = meter.u64_gauge(DB_DUMP_UPLOAD_DURATION_NS).build();

        Self {
            shared,
            background_jobs,
            background_jobs_oldest_age,
            crates_total,
            versions_total,
            db_dump_size,
            db_dump_upload_duration,
        }
    }

    /// Records the total crate and version counts.
    pub fn record_service_totals(&self, crates: i64, versions: i64) {
        self.crates_total.record(crates, &[]);
        self.versions_total.record(versions, &[]);
    }

    /// Records the count and oldest age of outstanding jobs for `queue`.
    pub fn record_background_jobs(
        &self,
        queue: &(String, String),
        count: i64,
        oldest_age: chrono::Duration,
    ) {
        let (priority, job) = queue;
        let attributes = [kv(PRIORITY, priority.clone()), kv(JOB, job.clone())];
        self.background_jobs.record(count, &attributes);
        let age = oldest_age.as_seconds_f64();
        self.background_jobs_oldest_age.record(age, &attributes);
    }

    /// Records the size and upload duration of a database dump archive.
    pub fn record_db_dump(&self, format: &'static str, size: u64, upload_duration: Duration) {
        let attributes = [kv(FORMAT, format)];
        let upload_ns = u64::try_from(upload_duration.as_nanos()).unwrap_or(u64::MAX);

        self.db_dump_size.record(size, &attributes);
        self.db_dump_upload_duration.record(upload_ns, &attributes);
    }
}
