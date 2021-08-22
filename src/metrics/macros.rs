use prometheus::{Histogram, HistogramOpts, HistogramVec, Opts};

/// Prometheus's histograms work by dividing datapoints in buckets, with each bucket containing
/// the count of datapoints equal or greater to the bucket value.
///
/// The buckets used by crates.io are geared towards measuring the response time of our requests,
/// going from 0.5ms to 100ms with a higher resolution and from 100ms to 5 seconds with a slightly
/// lower resolution. This allows us to properly measure download requests (which take around 1ms)
/// and other requests (our 95h is around 10-20ms).
///
/// Histogram buckets are not an exact science, so feel free to tweak the buckets if you see that
/// the histograms are not really accurate. Just avoid adding too many buckets as that increases
/// the number of exported metric series.
const HISTOGRAM_BUCKETS: &[f64] = &[
    0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.5, 1.0, 5.0,
];

pub(super) trait MetricFromOpts: Sized {
    fn from_opts(opts: Opts) -> Result<Self, prometheus::Error>;
}

#[macro_export]
macro_rules! metrics {
    (
        $vis:vis struct $name:ident {
            $(
                #[doc = $help:expr]
                $(#[$meta:meta])*
                $metric_vis:vis $metric:ident: $ty:ty $([$($label:expr),* $(,)?])?
            ),* $(,)?
        }
        namespace: $namespace:expr,
    ) => {
        $vis struct $name {
            registry: prometheus::Registry,
            $(
                $(#[$meta])*
                $metric_vis $metric: $ty,
            )*
        }
        impl $name {
            $vis fn new() -> Result<Self, prometheus::Error> {
                use crate::metrics::macros::MetricFromOpts;

                let registry = prometheus::Registry::new();
                $(
                    $(#[$meta])*
                    let $metric = <$ty>::from_opts(
                        prometheus::Opts::new(stringify!($metric), $help)
                            .namespace($namespace)
                            $(.variable_labels(vec![$($label.into()),*]))?
                    )?;
                    $(#[$meta])*
                    registry.register(Box::new($metric.clone()))?;
                )*
                Ok(Self {
                    registry,
                    $(
                        $(#[$meta])*
                        $metric,
                    )*
                })
            }
        }
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", stringify!($name))
            }
        }
    };
}

macro_rules! load_metric_type {
    ($name:ident as single) => {
        use prometheus::$name;
        impl MetricFromOpts for $name {
            fn from_opts(opts: Opts) -> Result<Self, prometheus::Error> {
                $name::with_opts(opts.into())
            }
        }
    };
    ($name:ident as vec) => {
        use prometheus::$name;
        impl MetricFromOpts for $name {
            fn from_opts(opts: Opts) -> Result<Self, prometheus::Error> {
                $name::new(
                    opts.clone().into(),
                    opts.variable_labels
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .as_slice(),
                )
            }
        }
    };
}

load_metric_type!(Counter as single);
load_metric_type!(CounterVec as vec);
load_metric_type!(IntCounter as single);
load_metric_type!(IntCounterVec as vec);
load_metric_type!(Gauge as single);
load_metric_type!(GaugeVec as vec);
load_metric_type!(IntGauge as single);
load_metric_type!(IntGaugeVec as vec);

// Use a custom implementation for histograms to customize the buckets.

impl MetricFromOpts for Histogram {
    fn from_opts(opts: Opts) -> Result<Self, prometheus::Error> {
        Histogram::with_opts(HistogramOpts {
            common_opts: opts,
            buckets: HISTOGRAM_BUCKETS.to_vec(),
        })
    }
}

impl MetricFromOpts for HistogramVec {
    fn from_opts(opts: Opts) -> Result<Self, prometheus::Error> {
        HistogramVec::new(
            HistogramOpts {
                common_opts: opts.clone(),
                buckets: HISTOGRAM_BUCKETS.to_vec(),
            },
            opts.variable_labels
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .as_slice(),
        )
    }
}
