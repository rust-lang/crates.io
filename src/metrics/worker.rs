use super::consts::{CRATES_TOTAL, VERSIONS_TOTAL};
use opentelemetry::metrics::{Gauge, Meter};

/// OpenTelemetry instruments recorded by the background worker.
#[derive(Clone, Debug)]
pub struct WorkerMetrics {
    crates_total: Gauge<i64>,
    versions_total: Gauge<i64>,
}

impl WorkerMetrics {
    /// Creates the worker instruments from `meter`.
    pub fn new(meter: &Meter) -> Self {
        let crates_total = meter.i64_gauge(CRATES_TOTAL).build();
        let versions_total = meter.i64_gauge(VERSIONS_TOTAL).build();

        Self {
            crates_total,
            versions_total,
        }
    }

    /// Records the total crate and version counts.
    pub fn record_service_totals(&self, crates: i64, versions: i64) {
        self.crates_total.record(crates, &[]);
        self.versions_total.record(versions, &[]);
    }
}
