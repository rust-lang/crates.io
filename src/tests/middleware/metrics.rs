use crate::util::{MockRequestExt, RequestHelper, TestApp};
use crates_io::metrics::consts::{
    HTTP_REQUEST_METHOD, HTTP_SERVER_ACTIVE_REQUESTS, METER_NAME, URL_SCHEME,
};
use http::{Method, StatusCode};
use opentelemetry::Value;
use opentelemetry::metrics::MeterProvider;
use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader, SdkMeterProvider};

#[tokio::test(flavor = "multi_thread")]
async fn reports_active_requests() {
    let exporter = InMemoryMetricExporter::default();
    let reader = PeriodicReader::builder(exporter.clone()).build();
    let provider = SdkMeterProvider::builder().with_reader(reader).build();
    let meter = provider.meter(METER_NAME);

    let (_, anon) = TestApp::init().with_meter(meter).empty().await;

    let mut request = anon.request_builder(Method::GET, "/unknown");
    request.header("x-forwarded-proto", "https");
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    provider.force_flush().unwrap();

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
    assert!(!sum.is_monotonic());

    let point = sum.data_points().next().unwrap();
    assert_eq!(point.value(), 0);
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
