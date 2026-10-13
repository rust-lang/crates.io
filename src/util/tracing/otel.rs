use crate::config;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing::warn;

/// Builds the configured trace provider, returning `None` when disabled or initialization fails.
///
/// The caller owns flushing and shutdown of the returned provider.
pub fn tracer_provider(config: &config::SharedConfig) -> Option<SdkTracerProvider> {
    if !config.otel.tracing_enabled {
        return None;
    }

    match otlp_tracer_provider(config) {
        Ok(provider) => Some(provider),
        Err(error) => {
            warn!("Failed to initialize OpenTelemetry tracing: {error:#}");
            None
        }
    }
}

/// Builds a provider with a batched OTLP/HTTP exporter, configured through the
/// SDK's standard environment variables.
fn otlp_tracer_provider(config: &config::SharedConfig) -> anyhow::Result<SdkTracerProvider> {
    let resource = crate::metrics::resource(config)?;
    let exporter = SpanExporter::builder().with_http().build()?;

    Ok(SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build())
}
