use crates_io_env_vars::var;
use secrecy::SecretString;

#[derive(Debug, Default)]
pub struct MetricsConfig {
    /// Whether OTLP metrics export was selected.
    ///
    /// Selected when either `OTEL_EXPORTER_OTLP_ENDPOINT` or
    /// `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT` is present.
    pub otlp_enabled: bool,

    /// Authorization token needed to query the metrics endpoints. If missing,
    /// querying metrics is completely disabled.
    ///
    /// Read from the `METRICS_AUTHORIZATION_TOKEN` environment variable.
    pub authorization_token: Option<SecretString>,
}

impl MetricsConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let otlp_endpoint = var("OTEL_EXPORTER_OTLP_ENDPOINT")?;
        let metrics_endpoint = var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT")?;
        let otlp_enabled = otlp_endpoint.is_some() || metrics_endpoint.is_some();
        let authorization_token = var("METRICS_AUTHORIZATION_TOKEN")?.map(Into::into);

        Ok(Self {
            otlp_enabled,
            authorization_token,
        })
    }
}
