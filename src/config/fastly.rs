use crates_io_env_vars::var;
use secrecy::SecretString;

/// Configuration for the Fastly API client.
#[derive(Debug)]
pub struct FastlyConfig {
    /// API token used to authenticate Fastly requests.
    ///
    /// Read from the `FASTLY_API_TOKEN` environment variable.
    pub api_token: SecretString,

    /// Service ID for `static.crates.io` (or equivalent).
    ///
    /// Read from the `FASTLY_SERVICE_ID_STATIC` environment variable.
    pub service_id_static: Option<String>,
}

impl FastlyConfig {
    /// Loads the optional Fastly configuration from environment variables.
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let Some(api_token) = var("FASTLY_API_TOKEN")? else {
            return Ok(None);
        };
        let service_id_static = var("FASTLY_SERVICE_ID_STATIC")?;

        Ok(Some(Self {
            api_token: api_token.into(),
            service_id_static,
        }))
    }
}
