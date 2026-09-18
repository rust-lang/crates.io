//! Records service metrics and optionally submits the legacy Prometheus values
//! to Datadog's [submit metrics API][api] (`POST /api/v2/series`).
//!
//! [`spawn`] starts the background task. The rest of the module encodes the
//! gathered Prometheus families into the JSON payload the API expects.
//!
//! [api]: https://docs.datadoghq.com/api/latest/metrics/

use crate::config::SharedConfig;
use crate::datadog::common_tags;
use crate::metrics::{ServiceMetrics, ServiceMetricsSnapshot, WorkerMetrics};
use anyhow::{Context, anyhow};
use crates_io_datadog::{DatadogClient, MetricType as DatadogMetricType, Point, Resource, Series};
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::deadpool::Pool;
use prometheus::proto::{MetricFamily, MetricType};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// Interval between service metric collections.
const COLLECT_INTERVAL: Duration = Duration::from_secs(5);

/// Spawns a background task that periodically records service metrics.
///
/// This must run in exactly one process. Today that is guaranteed by the single
/// `background_worker` dyno. Scaling the worker horizontally would report the
/// same service-wide values from every instance.
pub fn spawn(
    config: &SharedConfig,
    deadpool: Pool<AsyncPgConnection>,
    worker_metrics: WorkerMetrics,
    mut datadog: Option<Arc<DatadogClient>>,
) {
    if config.metrics.otlp_enabled {
        info!("OTLP metrics selected, skipping direct Datadog metrics submission");
        datadog = None;
    } else if datadog.is_none() {
        info!("Datadog API key not configured, skipping Datadog metrics submission");
    }

    let legacy = datadog.and_then(|datadog| {
        ServiceMetrics::new()
            .map(|metrics| (datadog, metrics))
            .inspect_err(|err| warn!("Failed to initialize service metrics: {err}"))
            .ok()
    });

    let domain_name = config.domain_name.clone();
    let mut common_tags = common_tags(&domain_name);
    if let Ok(Some(commit)) = crates_io_version::commit() {
        common_tags.push(format!("version:{commit}"));
    }
    let resources = vec![Resource::builder().kind("host").name(domain_name).build()];

    tokio::spawn(async move {
        let mut observed_queues = HashSet::new();

        loop {
            let result = submit(
                &deadpool,
                &worker_metrics,
                &mut observed_queues,
                legacy.as_ref(),
                &resources,
                &common_tags,
            )
            .await;

            if let Err(err) = result {
                warn!("Failed to record service metrics: {err}");
            }

            tokio::time::sleep(COLLECT_INTERVAL).await;
        }
    });
}

/// Records the service metrics and optionally submits them directly to Datadog.
async fn submit(
    deadpool: &Pool<AsyncPgConnection>,
    worker_metrics: &WorkerMetrics,
    observed_queues: &mut HashSet<(String, String)>,
    legacy: Option<&(Arc<DatadogClient>, ServiceMetrics)>,
    resources: &[Resource],
    common_tags: &[String],
) -> anyhow::Result<()> {
    let mut conn = deadpool
        .get()
        .await
        .context("Failed to acquire database connection")?;

    let snapshot = ServiceMetricsSnapshot::load(&mut conn)
        .await
        .map_err(|err| anyhow!("{err}"))
        .context("Failed to gather service metrics")?;

    worker_metrics.record_service_totals(snapshot.crates_total, snapshot.versions_total);

    observed_queues.extend(snapshot.background_jobs.keys().cloned());
    for queue in observed_queues.iter() {
        let count = snapshot.background_jobs.get(queue).copied().unwrap_or(0);
        worker_metrics.record_background_jobs(queue, count);
    }

    let Some((datadog, service_metrics)) = legacy else {
        return Ok(());
    };

    let families = service_metrics
        .record(snapshot)
        .context("Failed to gather service metrics")?;

    let timestamp = chrono::Utc::now().timestamp();
    let series = families_to_series(&families, timestamp, resources, common_tags);

    if let Err(err) = datadog.submit_metrics(&series).await {
        warn!("Failed to submit Datadog metrics: {err}");
    }

    Ok(())
}

/// Builds one Datadog [`Series`] per metric in the gathered families.
///
/// The `cratesio_service_` namespace prefix is rewritten to `crates_io.`, so
/// `cratesio_service_background_jobs` becomes `crates_io.background_jobs`. Each
/// series carries the metric's own labels concatenated with `common_tags` and
/// the shared `resources`.
///
/// Unsupported metric types are logged and skipped rather than panicking: a
/// metrics encoder must not crash the worker on an unexpected type.
fn families_to_series(
    families: &[MetricFamily],
    timestamp: i64,
    resources: &[Resource],
    common_tags: &[String],
) -> Vec<Series> {
    let mut series = Vec::new();

    for family in families {
        let name = family.name();
        let metric = match name.strip_prefix("cratesio_service_") {
            Some(rest) => format!("crates_io.{rest}"),
            None => name.to_string(),
        };

        for proto in family.get_metric() {
            let (kind, value) = match family.get_field_type() {
                MetricType::GAUGE => (DatadogMetricType::Gauge, proto.get_gauge().get_value()),
                MetricType::COUNTER => (DatadogMetricType::Count, proto.get_counter().get_value()),
                other => {
                    warn!("unsupported metric type: {other:?}");
                    continue;
                }
            };

            let mut tags = proto
                .get_label()
                .iter()
                .map(|l| format!("{}:{}", l.name(), l.value()))
                .collect::<Vec<_>>();

            tags.extend_from_slice(common_tags);

            let point = Point::builder().timestamp(timestamp).value(value).build();
            let series_item = Series::builder()
                .metric(metric.clone())
                .kind(kind)
                .points(vec![point])
                .resources(resources.to_vec())
                .tags(tags)
                .build();
            series.push(series_item);
        }
    }

    series
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Error;
    use prometheus::{Histogram, HistogramOpts, IntCounter, IntGaugeVec, Opts, Registry};

    fn host() -> Resource {
        Resource::builder().kind("host").name("crates.io").build()
    }

    #[test]
    fn test_families_to_series() -> Result<(), Error> {
        let registry = Registry::new();

        let gauge_vec = IntGaugeVec::new(
            Opts::new("background_jobs", "queued jobs").namespace("cratesio_service"),
            &["priority", "job"],
        )?;
        gauge_vec.with_label_values(&["1", "foo"]).set(42);
        gauge_vec.with_label_values(&["2", "bar"]).set(98);
        registry.register(Box::new(gauge_vec))?;

        let counter = IntCounter::with_opts(
            Opts::new("crates_total", "total crates").namespace("cratesio_service"),
        )?;
        counter.inc_by(7);
        registry.register(Box::new(counter))?;

        // A name without the `cratesio_service_` prefix passes through unchanged.
        let other = IntCounter::with_opts(Opts::new("other_metric", "help"))?;
        registry.register(Box::new(other))?;

        // Unsupported metric types are skipped instead of producing a series.
        let histogram = Histogram::with_opts(HistogramOpts::new("sample_histogram", "help"))?;
        histogram.observe(1.0);
        registry.register(Box::new(histogram))?;

        let resources = [host()];
        let common_tags = ["env:prod".to_string()];
        let series = families_to_series(&registry.gather(), 1000, &resources, &common_tags);

        // Gauges map to `type: 3`, counters to `type: 1`, and the
        // `cratesio_service_` prefix is rewritten to `crates_io.`
        insta::assert_json_snapshot!(series);

        Ok(())
    }
}
