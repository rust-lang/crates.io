use crate::config;
use opentelemetry::metrics::{MeterProvider, noop::NoopMeterProvider};
use opentelemetry::{KeyValue, Value};
use opentelemetry_otlp::MetricExporter;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use tracing::warn;
use uuid::Uuid;

/// Builds the configured meter provider.
///
/// A no-op provider is returned when OTLP is disabled or cannot be initialized.
pub fn meter_provider(config: &config::SharedConfig) -> Box<dyn MeterProvider + Send + Sync> {
    if !config.metrics.otlp_enabled {
        return Box::new(NoopMeterProvider::new());
    }

    match otlp_meter_provider(config) {
        Ok(provider) => Box::new(provider),
        Err(error) => {
            warn!("Failed to initialize OpenTelemetry metrics: {error}");
            Box::new(NoopMeterProvider::new())
        }
    }
}

fn otlp_meter_provider(config: &config::SharedConfig) -> anyhow::Result<SdkMeterProvider> {
    let resource = resource(config)?;
    let exporter = MetricExporter::builder().with_http().build()?;
    let reader = PeriodicReader::builder(exporter).build();
    let provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .build();

    Ok(provider)
}

fn resource(config: &config::SharedConfig) -> anyhow::Result<Resource> {
    let environment = match config.domain_name.as_str() {
        "staging.crates.io" => "staging",
        // This uses a non-standard value for backwards compatibility for now.
        _ => "prod",
    };
    let mut builder = Resource::builder().with_attributes([
        kv("service.name", "crates_io"),
        kv("deployment.environment.name", environment),
        kv("service.instance.id", Uuid::new_v4().to_string()),
    ]);

    if crates_io_heroku::is_heroku()? {
        builder = builder.with_attribute(kv("cloud.provider", "heroku"));

        if let Some(value) = crates_io_heroku::dyno_id()? {
            builder = builder.with_attribute(kv("service.instance.id", value));
        }
        if let Some(value) = crates_io_heroku::release_version()? {
            builder = builder.with_attribute(kv("service.version", value));
        }
        if let Some(value) = crates_io_heroku::app_id()? {
            builder = builder.with_attribute(kv("heroku.app.id", value));
        }
        if let Some(value) = crates_io_heroku::commit()? {
            builder = builder.with_attribute(kv("heroku.release.commit", value));
        }
        if let Some(value) = crates_io_heroku::release_created_at()? {
            builder = builder.with_attribute(kv("heroku.release.creation_timestamp", value));
        }
        if let Some(value) = crates_io_heroku::dyno()? {
            builder = builder.with_attribute(kv("dyno", value));
        }
    }

    Ok(builder.build())
}

fn kv(key: &'static str, value: impl Into<Value>) -> KeyValue {
    KeyValue::new(key, value)
}
