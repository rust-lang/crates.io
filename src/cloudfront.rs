use aws_credential_types::Credentials;
use aws_sdk_cloudfront::config::retry::RetryConfig;
use aws_sdk_cloudfront::config::{BehaviorVersion, Region};
use aws_sdk_cloudfront::types::{InvalidationBatch, Paths};
use aws_sdk_cloudfront::{Client, Config};
use tracing::{debug, instrument, warn};

pub struct CloudFront {
    client: Client,
    distribution_id: String,
}

impl CloudFront {
    pub fn from_environment() -> Option<Self> {
        let distribution_id = dotenvy::var("CLOUDFRONT_DISTRIBUTION").ok()?;
        let access_key = dotenvy::var("AWS_ACCESS_KEY").expect("missing AWS_ACCESS_KEY");
        let secret_key = dotenvy::var("AWS_SECRET_KEY").expect("missing AWS_SECRET_KEY");

        let credentials = Credentials::from_keys(access_key, secret_key, None);

        let config = Config::builder()
            .behavior_version(BehaviorVersion::v2025_01_17())
            .region(Region::new("us-east-1"))
            .credentials_provider(credentials)
            .retry_config(RetryConfig::standard().with_max_attempts(10))
            .build();

        let client = Client::from_conf(config);

        Some(Self {
            client,
            distribution_id,
        })
    }

    /// Invalidate a file on CloudFront
    ///
    /// `path` is the path to the file to invalidate, such as `config.json`, or `re/ge/regex`
    pub async fn invalidate(&self, path: &str) -> anyhow::Result<()> {
        self.invalidate_many(vec![path.to_string()]).await
    }

    /// Invalidate multiple paths on Cloudfront.
    #[instrument(skip(self))]
    pub async fn invalidate_many(&self, mut paths: Vec<String>) -> anyhow::Result<()> {
        let now = chrono::offset::Utc::now().timestamp_micros();

        // We need to ensure that paths have a starting slash.
        for path in paths.iter_mut() {
            if !path.starts_with('/') {
                *path = format!("/{path}");
            }
        }

        let paths = Paths::builder()
            // It looks like you have to set quantity even if you provide a full blown Vec, because
            // reasons.
            .quantity(paths.len() as i32)
            .set_items(Some(paths))
            .build()?;

        let invalidation_batch = InvalidationBatch::builder()
            .caller_reference(format!("{now}"))
            .paths(paths)
            .build()?;

        let invalidation_request = self
            .client
            .create_invalidation()
            .distribution_id(&self.distribution_id)
            .invalidation_batch(invalidation_batch);

        debug!("Sending invalidation request");

        match invalidation_request.send().await {
            Ok(_) => {
                debug!("Invalidation request successful");
                Ok(())
            }
            Err(error) => {
                warn!(?error, "Invalidation request failed");
                Err(error.into())
            }
        }
    }
}
