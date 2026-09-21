//! Periodically records database-backed service metrics.

use crate::metrics::{ServiceMetricsSnapshot, WorkerMetrics};
use anyhow::{Context, anyhow};
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::deadpool::Pool;
use std::collections::HashSet;
use std::time::Duration;
use tracing::warn;

/// Interval between service metric collections.
const COLLECT_INTERVAL: Duration = Duration::from_secs(5);

/// Spawns a background task that periodically records service metrics.
///
/// This must run in exactly one process. Today that is guaranteed by the single
/// `background_worker` dyno. Scaling the worker horizontally would report the
/// same service-wide values from every instance.
pub fn spawn(deadpool: Pool<AsyncPgConnection>, metrics: WorkerMetrics) {
    tokio::spawn(async move {
        let mut observed_queues = HashSet::new();

        loop {
            if let Err(error) = record(&deadpool, &metrics, &mut observed_queues).await {
                warn!("Failed to record service metrics: {error:#}");
            }

            tokio::time::sleep(COLLECT_INTERVAL).await;
        }
    });
}

async fn record(
    deadpool: &Pool<AsyncPgConnection>,
    metrics: &WorkerMetrics,
    observed_queues: &mut HashSet<(String, String)>,
) -> anyhow::Result<()> {
    let mut conn = deadpool
        .get()
        .await
        .context("Failed to acquire database connection")?;

    let snapshot = ServiceMetricsSnapshot::load(&mut conn)
        .await
        .map_err(|error| anyhow!("{error}"))
        .context("Failed to gather service metrics")?;

    metrics.record_service_totals(snapshot.crates_total, snapshot.versions_total);

    observed_queues.extend(snapshot.background_jobs.keys().cloned());
    for queue in observed_queues.iter() {
        let count = snapshot.background_jobs.get(queue).copied().unwrap_or(0);
        metrics.record_background_jobs(queue, count);
    }

    Ok(())
}
