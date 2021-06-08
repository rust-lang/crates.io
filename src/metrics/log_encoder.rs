use base64::write::EncoderWriter;
use indexmap::IndexMap;
use prometheus::proto::{MetricFamily, MetricType};
use prometheus::{Encoder, Error};
use std::io::Write;

/// The `LogEncoder` struct encodes Prometheus metrics in the format [`crates-io-heroku-metrics`]
/// expects metrics to be logged. This can be used to forward instance metrics to it, allowing them
/// to be scraped by the Rust infrastructure monitoring system.
///
/// The metrics are encoded in the format [Vector expects them][vector], and the list of metrics is
/// json-encoded and then base64-encoded. The whole thing is prefixed with a predefined string to
/// let [`crates-io-heroku-metrics`] find it easily.
///
/// This is needed mostly for crates.io hosted on Heroku. Deployments of crates.io on other
/// platforms shouldn't need this.
///
/// [`crates-io-heroku-metrics`]: https://github.com/rust-lang/crates-io-heroku-metrics
/// [vector]: https://vector.dev/docs/about/under-the-hood/architecture/data-model/metric/
#[derive(Debug, Clone, Copy)]
pub struct LogEncoder(());

impl LogEncoder {
    pub fn new() -> Self {
        Self(())
    }
}

impl Encoder for LogEncoder {
    fn encode<W: Write>(
        &self,
        families: &[MetricFamily],
        mut dest: &mut W,
    ) -> prometheus::Result<()> {
        let events = families_to_json_events(families);

        dest.write_all(b"crates-io-heroku-metrics:ingest ")?;
        let base64_dest = EncoderWriter::new(&mut dest, base64::STANDARD);
        serde_json::to_writer(base64_dest, &events).map_err(|e| Error::Msg(e.to_string()))?;
        dest.write_all(b"\n")?;

        Ok(())
    }

    fn format_type(&self) -> &str {
        "crates-io-heroku-metrics log encoding"
    }
}

fn families_to_json_events(families: &[MetricFamily]) -> Vec<VectorEvent<'_>> {
    let mut events = Vec::new();
    for family in families {
        for metric in family.get_metric() {
            let data = match family.get_field_type() {
                MetricType::COUNTER => VectorMetricData::Counter {
                    value: metric.get_counter().get_value(),
                },
                MetricType::GAUGE => VectorMetricData::Gauge {
                    value: metric.get_gauge().get_value(),
                },
                MetricType::HISTOGRAM => {
                    let histogram = metric.get_histogram();

                    // We need to convert from cumulative counts (used by the Prometheus library)
                    // to plain counts (used by Vector).
                    let mut buckets = Vec::new();
                    let mut last_cumulative_count = 0;
                    for bucket in histogram.get_bucket() {
                        buckets.push(VectorHistogramBucket {
                            upper_limit: bucket.get_upper_bound(),
                            count: bucket.get_cumulative_count() - last_cumulative_count,
                        });
                        last_cumulative_count = bucket.get_cumulative_count();
                    }

                    VectorMetricData::AggregatedHistogram {
                        count: histogram.get_sample_count(),
                        sum: histogram.get_sample_sum(),
                        buckets,
                    }
                }
                other => {
                    panic!("unsupported metric type: {:?}", other)
                }
            };
            events.push(VectorEvent {
                metric: VectorMetric {
                    data,
                    kind: "absolute",
                    name: family.get_name(),
                    tags: metric
                        .get_label()
                        .iter()
                        .map(|p| (p.get_name(), p.get_value()))
                        .collect(),
                },
            });
        }
    }
    events
}

#[derive(Serialize, Debug, PartialEq)]
struct VectorEvent<'a> {
    metric: VectorMetric<'a>,
}

#[derive(Serialize, Debug, PartialEq)]
struct VectorMetric<'a> {
    #[serde(flatten)]
    data: VectorMetricData,
    kind: &'a str,
    name: &'a str,
    tags: IndexMap<&'a str, &'a str>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
enum VectorMetricData {
    AggregatedHistogram {
        buckets: Vec<VectorHistogramBucket>,
        count: u64,
        sum: f64,
    },
    Counter {
        value: f64,
    },
    Gauge {
        value: f64,
    },
}

#[derive(Serialize, Debug, PartialEq)]
struct VectorHistogramBucket {
    upper_limit: f64,
    count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Error;
    use prometheus::{Histogram, HistogramOpts, IntCounter, IntGauge, IntGaugeVec, Opts, Registry};

    #[test]
    fn test_counter_to_json() -> Result<(), Error> {
        let counter =
            IntCounter::with_opts(Opts::new("sample_counter", "sample_counter help message"))?;
        let registry = Registry::new();
        registry.register(Box::new(counter.clone()))?;

        assert_eq!(
            vec![VectorEvent {
                metric: VectorMetric {
                    data: VectorMetricData::Counter { value: 0.0 },
                    kind: "absolute",
                    name: "sample_counter",
                    tags: IndexMap::new(),
                }
            }],
            families_to_json_events(&registry.gather())
        );

        counter.inc_by(42);
        assert_eq!(
            vec![VectorEvent {
                metric: VectorMetric {
                    data: VectorMetricData::Counter { value: 42.0 },
                    kind: "absolute",
                    name: "sample_counter",
                    tags: IndexMap::new(),
                }
            }],
            families_to_json_events(&registry.gather())
        );

        Ok(())
    }

    #[test]
    fn test_gauge_to_json() -> Result<(), Error> {
        let gauge = IntGauge::with_opts(Opts::new("sample_gauge", "sample_gauge help message"))?;
        let registry = Registry::new();
        registry.register(Box::new(gauge.clone()))?;

        assert_eq!(
            vec![VectorEvent {
                metric: VectorMetric {
                    data: VectorMetricData::Gauge { value: 0.0 },
                    kind: "absolute",
                    name: "sample_gauge",
                    tags: IndexMap::new(),
                }
            }],
            families_to_json_events(&registry.gather())
        );

        gauge.set(42);
        assert_eq!(
            vec![VectorEvent {
                metric: VectorMetric {
                    data: VectorMetricData::Gauge { value: 42.0 },
                    kind: "absolute",
                    name: "sample_gauge",
                    tags: IndexMap::new(),
                }
            }],
            families_to_json_events(&registry.gather())
        );

        Ok(())
    }

    #[test]
    fn test_histogram_to_json() -> Result<(), Error> {
        let histogram = Histogram::with_opts(HistogramOpts::new(
            "sample_histogram",
            "sample_histogram help message",
        ))?;
        let registry = Registry::new();
        registry.register(Box::new(histogram.clone()))?;

        let mut value = 0.0;
        while value < 11.0 {
            histogram.observe(value);
            value += 0.001;
        }

        assert_eq!(
            vec![VectorEvent {
                metric: VectorMetric {
                    data: VectorMetricData::AggregatedHistogram {
                        buckets: vec![
                            VectorHistogramBucket {
                                upper_limit: 0.005,
                                count: 6,
                            },
                            VectorHistogramBucket {
                                upper_limit: 0.01,
                                count: 4,
                            },
                            VectorHistogramBucket {
                                upper_limit: 0.025,
                                count: 15,
                            },
                            VectorHistogramBucket {
                                upper_limit: 0.05,
                                count: 25,
                            },
                            VectorHistogramBucket {
                                upper_limit: 0.1,
                                count: 50,
                            },
                            VectorHistogramBucket {
                                upper_limit: 0.25,
                                count: 150,
                            },
                            VectorHistogramBucket {
                                upper_limit: 0.5,
                                count: 250,
                            },
                            VectorHistogramBucket {
                                upper_limit: 1.0,
                                count: 500,
                            },
                            VectorHistogramBucket {
                                upper_limit: 2.5,
                                count: 1501,
                            },
                            VectorHistogramBucket {
                                upper_limit: 5.0,
                                count: 2499,
                            },
                            VectorHistogramBucket {
                                upper_limit: 10.0,
                                count: 5001,
                            },
                        ],
                        count: 11001,
                        sum: 60505.50000000138,
                    },
                    kind: "absolute",
                    name: "sample_histogram",
                    tags: IndexMap::new(),
                }
            }],
            families_to_json_events(&registry.gather())
        );

        Ok(())
    }

    #[test]
    fn test_metric_with_tags_to_json() -> Result<(), Error> {
        let gauge_vec = IntGaugeVec::new(
            Opts::new("sample_gauge", "sample_gauge help message"),
            &["label1", "label2"],
        )?;
        let registry = Registry::new();
        registry.register(Box::new(gauge_vec.clone()))?;

        gauge_vec.with_label_values(&["foo", "1"]).set(42);
        gauge_vec.with_label_values(&["bar", "2"]).set(98);

        assert_eq!(
            vec![
                VectorEvent {
                    metric: VectorMetric {
                        data: VectorMetricData::Gauge { value: 98.0 },
                        kind: "absolute",
                        name: "sample_gauge",
                        tags: [("label1", "bar"), ("label2", "2")]
                            .iter()
                            .copied()
                            .collect(),
                    }
                },
                VectorEvent {
                    metric: VectorMetric {
                        data: VectorMetricData::Gauge { value: 42.0 },
                        kind: "absolute",
                        name: "sample_gauge",
                        tags: [("label1", "foo"), ("label2", "1")]
                            .iter()
                            .copied()
                            .collect(),
                    }
                },
            ],
            families_to_json_events(&registry.gather())
        );

        Ok(())
    }

    #[test]
    fn test_encoding() -> Result<(), Error> {
        let gauge = IntGauge::with_opts(Opts::new("sample_gauge", "sample_gauge help message"))?;
        let registry = Registry::new();
        registry.register(Box::new(gauge.clone()))?;

        let mut output = Vec::new();
        LogEncoder::new().encode(&registry.gather(), &mut output)?;

        assert_eq!(
            b"crates-io-heroku-metrics:ingest W3sibWV0cmljIjp7ImdhdWdlIjp7InZhbHVlIjowLjB9LCJraW5kIjoiYWJzb2x1dGUiLCJuYW1lIjoic2FtcGxlX2dhdWdlIiwidGFncyI6e319fV0=\n",
            output.as_slice(),
        );

        Ok(())
    }
}
