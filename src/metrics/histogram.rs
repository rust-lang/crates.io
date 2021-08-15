use crate::metrics::macros::MetricFromOpts;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// Prometheus's histograms work by dividing datapoints in buckets, with each bucket containing the
/// count of datapoints equal or greater to the bucket value.
///
/// Histogram buckets are not an exact science, so feel free to tweak the buckets or create new
/// ones if you see that the histograms are not really accurate. Just avoid adding too many buckets
/// for a single type as that increases the number of exported metric series.
pub trait HistogramBuckets {
    const BUCKETS: &'static [f64];
}

/// Buckets geared towards measuring timings, such as the response time of our requests, going from
/// 0.5ms to 100ms with a higher resolution and from 100ms to 5 seconds with a slightly lower
/// resolution. This allows us to properly measure download requests (which take around 1ms) and
/// other requests (our 95h is around 10-20ms).
pub struct TimingBuckets;

impl HistogramBuckets for TimingBuckets {
    const BUCKETS: &'static [f64] = &[
        0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.5, 1.0, 5.0,
    ];
}

/// Wrapper type over [`prometheus::Histogram`] to support defining buckets.
pub struct Histogram<B: HistogramBuckets> {
    inner: prometheus::Histogram,
    _phantom: PhantomData<B>,
}

impl<B: HistogramBuckets> MetricFromOpts for Histogram<B> {
    fn from_opts(opts: prometheus::Opts) -> Result<Self, prometheus::Error> {
        Ok(Histogram {
            inner: prometheus::Histogram::with_opts(prometheus::HistogramOpts {
                common_opts: opts,
                buckets: B::BUCKETS.to_vec(),
            })?,
            _phantom: PhantomData,
        })
    }
}

impl<B: HistogramBuckets> Deref for Histogram<B> {
    type Target = prometheus::Histogram;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<B: HistogramBuckets> DerefMut for Histogram<B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Wrapper type over [`prometheus::HistogramVec`] to support defining buckets.
pub struct HistogramVec<B: HistogramBuckets> {
    inner: prometheus::HistogramVec,
    _phantom: PhantomData<B>,
}

impl<B: HistogramBuckets> MetricFromOpts for HistogramVec<B> {
    fn from_opts(opts: prometheus::Opts) -> Result<Self, prometheus::Error> {
        Ok(HistogramVec {
            inner: prometheus::HistogramVec::new(
                prometheus::HistogramOpts {
                    common_opts: opts.clone(),
                    buckets: B::BUCKETS.to_vec(),
                },
                opts.variable_labels
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .as_slice(),
            )?,
            _phantom: PhantomData,
        })
    }
}

impl<B: HistogramBuckets> Deref for HistogramVec<B> {
    type Target = prometheus::HistogramVec;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<B: HistogramBuckets> DerefMut for HistogramVec<B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
