//! Submits the `cratesio_service` metrics to Datadog's
//! [submit metrics API][api] (`POST /api/v2/series`).
//!
//! [`spawn`] starts a background task that periodically gathers the service
//! metrics and submits them. The rest of the module encodes the gathered
//! Prometheus families into the JSON payload the API expects.
//!
//! [api]: https://docs.datadoghq.com/api/latest/metrics/

use crate::config::Server;
use crate::metrics::ServiceMetrics;
use anyhow::{Context, anyhow};
use crates_io_datadog::{DatadogClient, MetricType as DatadogMetricType, Point, Resource, Series};
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::deadpool::Pool;
use prometheus::proto::{MetricFamily, MetricType};
use std::time::Duration;
use tracing::{info, warn};

/// Interval between metric submissions.
const SUBMIT_INTERVAL: Duration = Duration::from_secs(5);

/// Spawns a background task that periodically submits the service metrics to
/// Datadog.
///
/// This must run in exactly one process. Today that is guaranteed by the single
/// `background_worker` dyno; scaling the worker horizontally would make every
/// instance submit the same series (identical host and tags per environment),
/// causing last-write-wins collisions.
pub fn spawn(config: &Server, deadpool: Pool<AsyncPgConnection>, datadog: Option<DatadogClient>) {
    let Some(datadog) = datadog else {
        info!("Datadog API key not configured, skipping Datadog metrics submission");
        return;
    };

    let domain = config.domain_name.clone();
    let env = match domain.as_str() {
        "staging.crates.io" => "staging",
        _ => "prod",
    };
    let resources = vec![Resource::builder().kind("host").name(domain).build()];

    let mut common_tags = vec![format!("env:{env}"), "service:crates_io".into()];
    if let Ok(Some(commit)) = crates_io_version::commit() {
        common_tags.push(format!("version:{commit}"));
    }

    let service_metrics = match ServiceMetrics::new() {
        Ok(metrics) => metrics,
        Err(err) => {
            warn!("Failed to initialize service metrics: {err}");
            return;
        }
    };

    tokio::spawn(async move {
        loop {
            let result = submit(
                &deadpool,
                &service_metrics,
                &datadog,
                &resources,
                &common_tags,
            )
            .await;

            if let Err(err) = result {
                warn!("Failed to submit Datadog metrics: {err}");
            }

            tokio::time::sleep(SUBMIT_INTERVAL).await;
        }
    });
}

/// Gathers the service metrics and submits them to Datadog.
async fn submit(
    deadpool: &Pool<AsyncPgConnection>,
    service_metrics: &ServiceMetrics,
    datadog: &DatadogClient,
    resources: &[Resource],
    common_tags: &[String],
) -> anyhow::Result<()> {
    let mut conn = deadpool
        .get()
        .await
        .context("Failed to acquire database connection")?;

    let families = service_metrics
        .gather(&mut conn)
        .await
        .map_err(|err| anyhow!("{err}"))
        .context("Failed to gather service metrics")?;

    let timestamp = chrono::Utc::now().timestamp();
    let series = families_to_series(&families, timestamp, resources, common_tags);

    datadog.submit_metrics(&series).await?;

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
