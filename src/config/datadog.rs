use crates_io_datadog::DatadogClient;
use crates_io_env_vars::var;
use reqwest::Client;
use secrecy::SecretString;

const DEFAULT_SITE: &str = "datadoghq.com";

pub struct DatadogConfig {
    /// Datadog API key used to submit data. If missing, Datadog submissions
    /// are disabled.
    ///
    /// Read from the `DD_API_KEY` environment variable.
    pub api_key: Option<SecretString>,

    /// Datadog site to submit data to. Defaults to `datadoghq.com`.
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
    /// Creates a Datadog client when an API key is configured.
    pub fn client(&self, http_client: Client) -> Option<DatadogClient> {
        let api_key = self.api_key.clone()?;

        let client = DatadogClient::builder()
            .http_client(http_client)
            .api_key(api_key)
            .site(self.site.clone())
            .build();

        Some(client)
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let api_key = var("DD_API_KEY")?.map(SecretString::from);
        let site = var("DD_SITE")?.unwrap_or_else(|| DEFAULT_SITE.into());

        Ok(Self { api_key, site })
    }
}
