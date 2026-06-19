//! Encodes Prometheus metrics into the payload expected by Datadog's
//! [submit metrics API][api] (`POST /api/v2/series`).
//!
//! [api]: https://docs.datadoghq.com/api/latest/metrics/

#![cfg_attr(not(test), expect(dead_code))]

use prometheus::proto::{MetricFamily, MetricType};
use serde::Serialize;
use tracing::warn;

#[derive(Serialize, Debug, PartialEq)]
struct Series {
    metric: String,
    #[serde(rename = "type")]
    kind: u32,
    points: Vec<Point>,
    resources: Vec<Resource>,
    tags: Vec<String>,
}

#[derive(Serialize, Debug, PartialEq)]
struct Point {
    timestamp: i64,
    value: f64,
}

#[derive(Serialize, Debug, PartialEq, Clone)]
struct Resource {
    #[serde(rename = "type")]
    kind: String,
    name: String,
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
                MetricType::GAUGE => (3, proto.get_gauge().get_value()),
                MetricType::COUNTER => (1, proto.get_counter().get_value()),
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

            series.push(Series {
                metric: metric.clone(),
                kind,
                points: vec![Point { timestamp, value }],
                resources: resources.to_vec(),
                tags,
            });
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
        Resource {
            kind: "host".into(),
            name: "crates.io".into(),
        }
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
