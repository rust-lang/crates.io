use crates_io_env_vars::var;
use opentelemetry_otlp::{OTEL_EXPORTER_OTLP_ENDPOINT, OTEL_EXPORTER_OTLP_METRICS_ENDPOINT};

#[derive(Debug, Default)]
pub struct MetricsConfig {
    /// Whether OTLP metrics export was selected.
    ///
    /// Selected when either `OTEL_EXPORTER_OTLP_ENDPOINT` or
    /// `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT` is present.
    pub otlp_enabled: bool,
}

impl MetricsConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let otlp_endpoint = var(OTEL_EXPORTER_OTLP_ENDPOINT)?;
        let metrics_endpoint = var(OTEL_EXPORTER_OTLP_METRICS_ENDPOINT)?;
        let otlp_enabled = otlp_endpoint.is_some() || metrics_endpoint.is_some();

        Ok(Self { otlp_enabled })
    }
}
