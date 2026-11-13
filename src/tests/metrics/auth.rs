use crate::builders::{CrateBuilder, PublishBuilder, VersionBuilder};
use crate::util::{MockTokenUser, RequestHelper, TestApp};
use chrono::{TimeDelta, Utc};
use crates_io::metrics::consts::{AUTH_CHECKS, METER_NAME};
use crates_io::models::token::{CrateScope, EndpointScope};
use crates_io::schema::crates;
use crates_io_database::models::trustpub::NewToken;
use crates_io_trustpub::access_token::AccessToken;
use diesel::ExpressionMethods;
use diesel_async::RunQueryDsl;
use insta::assert_snapshot;
use opentelemetry::metrics::{Meter, MeterProvider};
use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader, SdkMeterProvider};
use secrecy::ExposeSecret;

struct MetricsFixture {
    exporter: InMemoryMetricExporter,
    provider: SdkMeterProvider,
}

impl MetricsFixture {
    fn new() -> (Self, Meter) {
        let exporter = InMemoryMetricExporter::default();
        let reader = PeriodicReader::builder(exporter.clone()).build();
        let provider = SdkMeterProvider::builder().with_reader(reader).build();
        let meter = provider.meter(METER_NAME);
        (Self { exporter, provider }, meter)
    }

    /// Returns one sorted line per `crates_io.auth.checks` data point, with
    /// its count followed by its sorted attributes.
    fn auth_checks(&self) -> String {
        self.provider.force_flush().unwrap();

        let batches = self.exporter.get_finished_metrics().unwrap();
        let Some(metric) = batches
            .iter()
            .flat_map(|batch| batch.scope_metrics())
            .flat_map(|scope| scope.metrics())
            .filter(|metric| metric.name() == AUTH_CHECKS)
            .last()
        else {
            return String::new();
        };

        let AggregatedMetrics::U64(MetricData::Sum(sum)) = metric.data() else {
            panic!("auth checks should be a u64 sum");
        };
        assert!(sum.is_monotonic());

        let mut lines = sum
            .data_points()
            .map(|point| {
                let attributes = point
                    .attributes()
                    .map(|kv| format!("{}={}", kv.key, kv.value));
                let mut attributes = attributes.collect::<Vec<_>>();
                attributes.sort();
                format!("{}: {}", point.value(), attributes.join(" "))
            })
            .collect::<Vec<_>>();

        lines.sort();
        lines.join("\n")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn cookie_and_legacy_token() -> anyhow::Result<()> {
    let (metrics, meter) = MetricsFixture::new();
    let (app, _, cookie, token) = TestApp::full().with_meter(meter).with_token().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .version(VersionBuilder::new("1.0.0"))
        .build(&mut conn)
        .await?;

    let response = cookie.delete::<()>("/api/v1/crates/foo/1.0.0/yank").await;
    assert_snapshot!(response.status(), @"200 OK");

    let response = token.put::<()>("/api/v1/crates/foo/1.0.0/unyank", "").await;
    assert_snapshot!(response.status(), @"200 OK");

    assert_snapshot!(metrics.auth_checks(), @"
    1: crates_io.auth.endpoint_scope=yank crates_io.auth.type=cookie http.request.method=DELETE http.route=/api/v1/crates/{name}/{version}/yank
    1: crates_io.auth.endpoint_scope=yank crates_io.auth.type=legacy_token http.request.method=PUT http.route=/api/v1/crates/{name}/{version}/unyank
    ");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn anonymous_requests_are_not_counted() {
    let (metrics, meter) = MetricsFixture::new();
    let (_, anon) = TestApp::init().with_meter(meter).empty().await;

    let response = anon.get::<()>("/api/v1/me/updates").await;
    assert_snapshot!(response.status(), @"403 Forbidden");

    assert_snapshot!(metrics.auth_checks(), @"");
}

#[tokio::test(flavor = "multi_thread")]
async fn scoped_tokens() -> anyhow::Result<()> {
    let (metrics, meter) = MetricsFixture::new();
    let (app, _, user) = TestApp::full().with_meter(meter).with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", user.as_model().id)
        .version(VersionBuilder::new("1.0.0"))
        .build(&mut conn)
        .await?;

    let yank = Some(vec![EndpointScope::Yank]);
    let publish = Some(vec![EndpointScope::PublishUpdate]);
    let other_crate = Some(vec![CrateScope::try_from("bar").unwrap()]);

    let url = "/api/v1/crates/foo/1.0.0/yank";

    let token = user
        .db_new_scoped_token("yank", None, yank.clone(), None)
        .await;
    let response = token.delete::<()>(url).await;
    assert_snapshot!(response.status(), @"200 OK");

    let token = user
        .db_new_scoped_token("publish", None, publish, None)
        .await;
    let response = token.delete::<()>(url).await;
    assert_snapshot!(response.status(), @"403 Forbidden");

    let token = user
        .db_new_scoped_token("bar", other_crate, yank, None)
        .await;
    let response = token.delete::<()>(url).await;
    assert_snapshot!(response.status(), @"403 Forbidden");

    assert_snapshot!(metrics.auth_checks(), @"
    1: crates_io.auth.endpoint_scope=yank crates_io.auth.type=scoped_token error.type=crate_scope_mismatch http.request.method=DELETE http.route=/api/v1/crates/{name}/{version}/yank
    1: crates_io.auth.endpoint_scope=yank crates_io.auth.type=scoped_token error.type=endpoint_scope_mismatch http.request.method=DELETE http.route=/api/v1/crates/{name}/{version}/yank
    1: crates_io.auth.endpoint_scope=yank crates_io.auth.type=scoped_token http.request.method=DELETE http.route=/api/v1/crates/{name}/{version}/yank
    ");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn cookie_only_endpoint_with_token() {
    let (metrics, meter) = MetricsFixture::new();
    let (_, _, _, token) = TestApp::init().with_meter(meter).with_token().await;

    let response = token.get::<()>("/api/v1/me").await;
    assert_snapshot!(response.status(), @"403 Forbidden");

    assert_snapshot!(metrics.auth_checks(), @"1: crates_io.auth.type=legacy_token error.type=cookie_required http.request.method=GET http.route=/api/v1/me");
}

#[tokio::test(flavor = "multi_thread")]
async fn trustpub_only_crate_with_token() -> anyhow::Result<()> {
    let (metrics, meter) = MetricsFixture::new();
    let (app, _, user, token) = TestApp::full().with_meter(meter).with_token().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", user.as_model().id)
        .build(&mut conn)
        .await?;

    diesel::update(crates::table)
        .filter(crates::name.eq("foo"))
        .set(crates::trustpub_only.eq(true))
        .execute(&mut conn)
        .await?;

    let response = token
        .publish_crate(PublishBuilder::new("foo", "1.1.0"))
        .await;
    assert_snapshot!(response.status(), @"403 Forbidden");

    assert_snapshot!(metrics.auth_checks(), @"1: crates_io.auth.endpoint_scope=publish-update crates_io.auth.type=legacy_token error.type=trustpub_required http.request.method=PUT http.route=/api/v1/crates/new");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn legacy_token_rejected() -> anyhow::Result<()> {
    let (metrics, meter) = MetricsFixture::new();
    let (app, _, user, token) = TestApp::full().with_meter(meter).with_token().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", user.as_model().id)
        .build(&mut conn)
        .await?;

    let body = serde_json::json!({ "crate": { "trustpub_only": true } }).to_string();
    let response = token.patch::<()>("/api/v1/crates/foo", body).await;
    assert_snapshot!(response.status(), @"403 Forbidden");

    assert_snapshot!(metrics.auth_checks(), @"1: crates_io.auth.endpoint_scope=trusted-publishing crates_io.auth.type=legacy_token http.request.method=PATCH http.route=/api/v1/crates/{name}");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn trusted_publishing() -> anyhow::Result<()> {
    let (metrics, meter) = MetricsFixture::new();
    let (app, _, user) = TestApp::full().with_meter(meter).with_user().await;
    let mut conn = app.db_conn().await;

    let krate = CrateBuilder::new("foo", user.as_model().id)
        .build(&mut conn)
        .await?;

    let token = AccessToken::generate();
    let hashed_token = token.sha256();
    let new_token = NewToken {
        expires_at: Utc::now() + TimeDelta::minutes(30),
        hashed_token: hashed_token.as_slice(),
        crate_ids: &[krate.id],
        trustpub_data: None,
    };
    new_token.insert(&conn).await?;

    let plaintext = token.finalize().expose_secret().to_string();
    let client = MockTokenUser::with_auth_header(plaintext, app);

    let pb = PublishBuilder::new("foo", "1.1.0");
    let response = client.publish_crate(pb).await;
    assert_snapshot!(response.status(), @"200 OK");

    assert_snapshot!(metrics.auth_checks(), @"1: crates_io.auth.type=trustpub http.request.method=PUT http.route=/api/v1/crates/new");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn trusted_publishing_with_other_crate() -> anyhow::Result<()> {
    let (metrics, meter) = MetricsFixture::new();
    let (app, _, user) = TestApp::full().with_meter(meter).with_user().await;
    let mut conn = app.db_conn().await;

    let foo = CrateBuilder::new("foo", user.as_model().id)
        .build(&mut conn)
        .await?;

    CrateBuilder::new("bar", user.as_model().id)
        .build(&mut conn)
        .await?;

    let token = AccessToken::generate();
    let hashed_token = token.sha256();
    let new_token = NewToken {
        expires_at: Utc::now() + TimeDelta::minutes(30),
        hashed_token: hashed_token.as_slice(),
        crate_ids: &[foo.id],
        trustpub_data: None,
    };
    new_token.insert(&conn).await?;

    let plaintext = token.finalize().expose_secret().to_string();
    let client = MockTokenUser::with_auth_header(plaintext, app);

    let response = client
        .publish_crate(PublishBuilder::new("bar", "1.1.0"))
        .await;
    assert_snapshot!(response.status(), @"403 Forbidden");

    assert_snapshot!(metrics.auth_checks(), @"1: crates_io.auth.type=trustpub error.type=trustpub_crate_mismatch http.request.method=PUT http.route=/api/v1/crates/new");

    Ok(())
}
