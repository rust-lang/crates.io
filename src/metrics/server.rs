use super::SharedMetrics;
use super::consts::{
    DB_CLIENT_CONNECTION_FALLBACKS, DB_CLIENT_CONNECTION_POOL_NAME, HTTP_REQUEST_METHOD,
    HTTP_SERVER_ACTIVE_REQUESTS, URL_SCHEME,
};
use super::otel::kv;
use derive_more::Deref;
use opentelemetry::KeyValue;
use opentelemetry::metrics::{Counter, Meter, UpDownCounter};

/// OpenTelemetry instruments recorded by the HTTP server.
#[derive(Clone, Debug, Deref)]
pub struct ServerMetrics {
    #[deref]
    shared: SharedMetrics,

    active_requests: UpDownCounter<i64>,
    db_fallbacks: Counter<u64>,
}

impl ServerMetrics {
    /// Creates the server instruments from `meter`.
    pub fn new(meter: &Meter) -> Self {
        let shared = SharedMetrics::new(meter);
        let active_requests = meter
            .i64_up_down_counter(HTTP_SERVER_ACTIVE_REQUESTS)
            .with_description("Number of active HTTP server requests.")
            .with_unit("{request}")
            .build();

        let db_fallbacks = meter
            .u64_counter(DB_CLIENT_CONNECTION_FALLBACKS)
            .with_unit("{fallback}")
            .build();

        Self {
            shared,
            active_requests,
            db_fallbacks,
        }
    }

    /// Records an active request until the returned guard is dropped.
    #[must_use]
    pub fn active_request(&self, method: &http::Method, scheme: &'static str) -> impl Drop + '_ {
        let attributes = [
            KeyValue::new(HTTP_REQUEST_METHOD, method.as_str().to_owned()),
            KeyValue::new(URL_SCHEME, scheme),
        ];
        ActiveRequestGuard::new(&self.active_requests, attributes)
    }

    /// Records a fallback from an unavailable database pool.
    pub fn db_fallback(&self, pool: &'static str) {
        self.db_fallbacks
            .add(1, &[kv(DB_CLIENT_CONNECTION_POOL_NAME, pool)]);
    }
}

#[derive(Debug)]
struct ActiveRequestGuard<'a> {
    counter: &'a UpDownCounter<i64>,
    attributes: [KeyValue; 2],
}

impl<'a> ActiveRequestGuard<'a> {
    fn new(counter: &'a UpDownCounter<i64>, attributes: [KeyValue; 2]) -> Self {
        counter.add(1, &attributes);
        Self {
            counter,
            attributes,
        }
    }
}

impl Drop for ActiveRequestGuard<'_> {
    fn drop(&mut self) {
        self.counter.add(-1, &self.attributes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::Value;
    use opentelemetry::metrics::MeterProvider;
    use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
    use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader, SdkMeterProvider};

    #[test]
    fn tracks_request_until_guard_is_dropped() {
        let exporter = InMemoryMetricExporter::default();
        let reader = PeriodicReader::builder(exporter.clone()).build();
        let provider = SdkMeterProvider::builder().with_reader(reader).build();
        let metrics = ServerMetrics::new(&provider.meter("test"));

        let guard = metrics.active_request(&http::Method::GET, "https");
        provider.force_flush().unwrap();
        assert_active_requests(&exporter, 1);

        drop(guard);
        provider.force_flush().unwrap();
        assert_active_requests(&exporter, 0);
    }

    fn assert_active_requests(exporter: &InMemoryMetricExporter, expected: i64) {
        let batches = exporter.get_finished_metrics().unwrap();
        let metric = batches
            .iter()
            .flat_map(|batch| batch.scope_metrics())
            .flat_map(|scope| scope.metrics())
            .filter(|metric| metric.name() == HTTP_SERVER_ACTIVE_REQUESTS)
            .last()
            .unwrap();

        let AggregatedMetrics::I64(MetricData::Sum(sum)) = metric.data() else {
            panic!("active requests should be an i64 sum");
        };

        let point = sum.data_points().next().unwrap();
        assert_eq!(point.value(), expected);
        assert_eq!(point.attributes().count(), 2);

        let method = point
            .attributes()
            .find(|attribute| attribute.key.as_str() == HTTP_REQUEST_METHOD)
            .unwrap();
        assert_eq!(method.value, Value::from("GET"));

        let scheme = point
            .attributes()
            .find(|attribute| attribute.key.as_str() == URL_SCHEME)
            .unwrap();
        assert_eq!(scheme.value, Value::from("https"));
    }
}
