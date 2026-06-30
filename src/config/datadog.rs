use crates_io_env_vars::var;
use secrecy::SecretString;

const DEFAULT_SITE: &str = "datadoghq.com";

pub struct DatadogConfig {
    /// Datadog API key used to submit service metrics. If missing, the
    /// background worker does not push metrics to Datadog.
    ///
    /// Read from the `DD_API_KEY` environment variable.
    pub api_key: Option<SecretString>,

    /// Datadog site to submit metrics to. Defaults to `datadoghq.com`.
    ///
    /// Read from the `DD_SITE` environment variable.
    pub site: String,
}

impl Default for DatadogConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            site: DEFAULT_SITE.into(),
        }
    }
}

impl DatadogConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let api_key = var("DD_API_KEY")?.map(SecretString::from);
        let site = var("DD_SITE")?.unwrap_or_else(|| DEFAULT_SITE.into());

        Ok(Self { api_key, site })
    }
}
