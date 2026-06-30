use crates_io_env_vars::{var, var_parsed};
use secrecy::SecretString;

#[derive(Debug, Default)]
pub struct MetricsConfig {
    /// Authorization token needed to query the metrics endpoints. If missing,
    /// querying metrics is completely disabled.
    ///
    /// Read from the `METRICS_AUTHORIZATION_TOKEN` environment variable.
    pub authorization_token: Option<SecretString>,

    /// How frequently instance metrics are logged, in seconds. If missing,
    /// instance metrics are not logged.
    ///
    /// Read from the `INSTANCE_METRICS_LOG_EVERY_SECONDS` environment variable.
    pub instance_log_every_seconds: Option<u64>,
}

impl MetricsConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let authorization_token = var("METRICS_AUTHORIZATION_TOKEN")?.map(Into::into);
        let instance_log_every_seconds = var_parsed("INSTANCE_METRICS_LOG_EVERY_SECONDS")?;

        Ok(Self {
            authorization_token,
            instance_log_every_seconds,
        })
    }
}
