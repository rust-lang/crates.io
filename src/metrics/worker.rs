use super::SharedMetrics;
use super::consts::{BACKGROUND_JOBS, CRATES_TOTAL, JOB, PRIORITY, VERSIONS_TOTAL};
use super::otel::kv;
use derive_more::Deref;
use opentelemetry::metrics::{Gauge, Meter};

/// OpenTelemetry instruments recorded by the background worker.
#[derive(Clone, Debug, Deref)]
pub struct WorkerMetrics {
    #[deref]
    shared: SharedMetrics,

    background_jobs: Gauge<i64>,
    crates_total: Gauge<i64>,
    versions_total: Gauge<i64>,
}

impl WorkerMetrics {
    /// Creates the worker instruments from `meter`.
    pub fn new(meter: &Meter) -> Self {
        let shared = SharedMetrics::new(meter);
        let background_jobs = meter.i64_gauge(BACKGROUND_JOBS).build();
        let crates_total = meter.i64_gauge(CRATES_TOTAL).build();
        let versions_total = meter.i64_gauge(VERSIONS_TOTAL).build();

        Self {
            shared,
            background_jobs,
            crates_total,
            versions_total,
        }
    }

    /// Records the total crate and version counts.
    pub fn record_service_totals(&self, crates: i64, versions: i64) {
        self.crates_total.record(crates, &[]);
        self.versions_total.record(versions, &[]);
    }

    /// Records the number of queued jobs for `queue`.
    pub fn record_background_jobs(&self, queue: &(String, String), count: i64) {
        let (priority, job) = queue;
        let attributes = [kv(PRIORITY, priority.clone()), kv(JOB, job.clone())];
        self.background_jobs.record(count, &attributes);
    }
}
