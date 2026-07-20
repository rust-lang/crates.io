use crates_io_env_vars::{required_var, var};
use secrecy::SecretString;
use tracing::warn;

/// Generic configuration for a queue that can be backed by SQS or mocked.
#[derive(Debug, Clone)]
pub enum QueueConfig {
    SQS {
        access_key: String,
        secret_key: SecretString,
        queue_url: String,
        region: String,
    },
    Mock,
}

impl QueueConfig {
    /// Build the configuration from the environment.
    ///
    /// The prefix is prepended to four environment variables. Using `QUEUE` as
    /// an example, we would expect the following environment variables:
    ///
    /// - `QUEUE_URL`
    /// - `QUEUE_ACCESS_KEY`
    /// - `QUEUE_SECRET_KEY`
    /// - `QUEUE_REGION`
    ///
    /// If `QUEUE_URL` doesn't exist, then a mock configuration is used. If it
    /// does exist, then all environment variables are expected, and this will
    /// return an error if one or more is missing.
    pub fn from_env(prefix: &str) -> anyhow::Result<Self> {
        if let Some(queue_url) = prefixed_var(prefix, "URL")? {
            let access_key = prefixed_required_var(prefix, "ACCESS_KEY")?;
            let secret_key = prefixed_required_var(prefix, "SECRET_KEY")?.into();
            let region = prefixed_required_var(prefix, "REGION")?;

            Ok(Self::SQS {
                access_key,
                secret_key,
                queue_url,
                region,
            })
        } else {
            warn!("Falling back to mocked {prefix} queue");
            Ok(Self::Mock)
        }
    }
}

fn prefixed_var(prefix: &str, suffix: &str) -> anyhow::Result<Option<String>> {
    var(&format!("{prefix}_{suffix}"))
}

fn prefixed_required_var(prefix: &str, suffix: &str) -> anyhow::Result<String> {
    required_var(&format!("{prefix}_{suffix}"))
}
