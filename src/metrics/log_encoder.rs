use base64::write::EncoderWriter;
use indexmap::IndexMap;
use prometheus::proto::{MetricFamily, MetricType};
use prometheus::{Encoder, Error};
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer as _};
use serde_json::Serializer;
use std::cell::Cell;
use std::io::Write;
use std::rc::Rc;

const CHUNKS_MAX_SIZE_BYTES: usize = 5000;

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
    fn encode<W: Write>(&self, families: &[MetricFamily], dest: &mut W) -> prometheus::Result<()> {
        let events = families_to_json_events(families);

        let chunks = serialize_and_split_list(events.iter(), CHUNKS_MAX_SIZE_BYTES)
            .map_err(|e| Error::Msg(e.to_string()))?;

        for chunk in chunks {
            dest.write_all(b"crates-io-heroku-metrics:ingest ")?;
            dest.write_all(&chunk)?;
            dest.write_all(b"\n")?;
        }

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

/// Serialize a list of items into multiple Base64-encoded JSON chunks.
///
/// Our hosting platform (Heroku) limits the size of log lines, arbitrarily splitting them once
/// they reach a threshold. We can't let Heroku do the split as it doesn't know where to properly
/// do that, so we need to do the splitting ourselves.
///
/// This function takes an iterator of serializable items and returns the serialized version,
/// possibly split into multiple chunks. Each chunk is *at least* `max_size_hint` long, as the
/// function stops serializing new items in the same chunk only when the size limit is reached
/// after serializing an item.
///
/// Because of that `max_size_hint` should be lower than the upper bound we can't cross.
fn serialize_and_split_list<'a, S: Serialize + 'a>(
    items: impl Iterator<Item = &'a S>,
    max_size_hint: usize,
) -> Result<Vec<Vec<u8>>, serde_json::Error> {
    let mut items = items.peekable();

    let mut result = Vec::new();
    while items.peek().is_some() {
        let mut writer = TrackedWriter::new();
        let written_count = writer.written_count.clone();
        let mut serializer = Serializer::new(EncoderWriter::new(&mut writer, base64::STANDARD));

        let mut seq = serializer.serialize_seq(None)?;
        while let Some(next) = items.next() {
            seq.serialize_element(next)?;
            if written_count.get() >= max_size_hint {
                break;
            }
        }
        seq.end()?;
        drop(serializer);

        result.push(writer.buffer);
    }

    Ok(result)
}

struct TrackedWriter {
    buffer: Vec<u8>,
    written_count: Rc<Cell<usize>>,
}

impl TrackedWriter {
    fn new() -> Self {
        Self {
            buffer: Vec::new(),
            written_count: Rc::new(Cell::new(0)),
        }
    }
}

impl Write for TrackedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.buffer.write(buf)?;
        self.written_count.set(self.written_count.get() + written);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.buffer.flush()
    }
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

    #[test]
    fn test_serialize_and_split_list_small() -> Result<(), Error> {
        let small = (0..10).collect::<Vec<_>>();
        let chunks = serialize_and_split_list(small.iter(), 256)?;

        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].len() <= 256);
        assert_eq!(
            serde_json::from_slice::<Vec<usize>>(&base64::decode(&chunks[0])?)?,
            small,
        );

        Ok(())
    }

    #[test]
    fn test_serialize_and_split_list_long() -> Result<(), Error> {
        let small = (0..100).collect::<Vec<_>>();
        let chunks = serialize_and_split_list(small.iter(), 256)?;

        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].len() >= 256);
        assert!(chunks[1].len() <= 256);
        assert_eq!(
            serde_json::from_slice::<Vec<usize>>(&base64::decode(&chunks[0])?)?,
            (0..=67).collect::<Vec<_>>(),
        );
        assert_eq!(
            serde_json::from_slice::<Vec<usize>>(&base64::decode(&chunks[1])?)?,
            (68..100).collect::<Vec<_>>(),
        );

        Ok(())
    }
}
