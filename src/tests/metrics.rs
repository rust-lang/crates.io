use crate::util::TestApp;
use crates_io::metrics::consts::{
    DB_CLIENT_CONNECTION_COUNT, DB_CLIENT_CONNECTION_POOL_NAME, DB_CLIENT_CONNECTION_STATE,
    METER_NAME,
};
use opentelemetry::Value;
use opentelemetry::metrics::MeterProvider;
use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData, SumDataPoint};
use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader, SdkMeterProvider};

#[tokio::test(flavor = "multi_thread")]
async fn reports_database_pool_connections() {
    let exporter = InMemoryMetricExporter::default();
    let reader = PeriodicReader::builder(exporter.clone()).build();
    let provider = SdkMeterProvider::builder().with_reader(reader).build();
    let meter = provider.meter(METER_NAME);

    let (app, _) = TestApp::full()
        .with_meter(meter)
        .with_replica()
        .empty()
        .await;
    let _connection = app.as_inner().primary_database.get().await.unwrap();
    provider.force_flush().unwrap();

    let batches = exporter.get_finished_metrics().unwrap();
    let metric = batches
        .iter()
        .flat_map(|batch| batch.scope_metrics())
        .flat_map(|scope| scope.metrics())
        .find(|metric| metric.name() == DB_CLIENT_CONNECTION_COUNT)
        .unwrap();

    let AggregatedMetrics::I64(MetricData::Sum(sum)) = metric.data() else {
        panic!("database pool connections should be an i64 sum");
    };
    assert!(!sum.is_monotonic());

    let points = sum.data_points().collect::<Vec<_>>();
    assert_eq!(points.len(), 6);
    assert_point(&points, "primary", "idle", 0);
    assert_point(&points, "primary", "used", 1);
    assert_point(&points, "replica", "idle", 0);
    assert_point(&points, "replica", "used", 0);
    assert_point(&points, "worker", "idle", 0);
    assert_point(&points, "worker", "used", 1);
}

fn assert_point(points: &[&SumDataPoint<i64>], pool: &str, state: &str, expected: i64) {
    let point = points
        .iter()
        .find(|point| {
            attribute(point, DB_CLIENT_CONNECTION_POOL_NAME) == Value::from(pool.to_owned())
                && attribute(point, DB_CLIENT_CONNECTION_STATE) == Value::from(state.to_owned())
        })
        .unwrap();
    assert_eq!(point.value(), expected);
}

fn attribute(point: &SumDataPoint<i64>, key: &str) -> Value {
    point
        .attributes()
        .find(|attribute| attribute.key.as_str() == key)
        .unwrap()
        .value
        .clone()
}
