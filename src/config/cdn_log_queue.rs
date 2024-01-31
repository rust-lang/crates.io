use crates_io_env_vars::{required_var, var};
use secrecy::SecretString;

#[derive(Debug, Clone)]
pub enum CdnLogQueueConfig {
    SQS {
        access_key: String,
        secret_key: SecretString,
        queue_url: String,
        region: String,
    },
    Mock,
}

impl CdnLogQueueConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        if let Some(queue_url) = var("CDN_LOG_QUEUE_URL")? {
            let access_key = required_var("CDN_LOG_QUEUE_ACCESS_KEY")?;
            let secret_key = required_var("CDN_LOG_QUEUE_SECRET_KEY")?.into();
            let region = required_var("CDN_LOG_QUEUE_REGION")?;

            return Ok(Self::SQS {
                access_key,
                secret_key,
                queue_url,
                region,
            });
        }

        warn!("Falling back to mocked CDN log queue");
        Ok(Self::Mock)
    }
}
