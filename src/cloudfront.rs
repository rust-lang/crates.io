use aws_credential_types::Credentials;
use aws_sdk_cloudfront::config::retry::RetryConfig;
use aws_sdk_cloudfront::config::{BehaviorVersion, Region};
use aws_sdk_cloudfront::types::{InvalidationBatch, Paths};
use aws_sdk_cloudfront::{Client, Config};

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
            .behavior_version(BehaviorVersion::v2023_11_09())
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
    #[instrument(skip(self))]
    pub async fn invalidate(&self, path: &str) -> anyhow::Result<()> {
        let path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };

        let now = chrono::offset::Utc::now().timestamp_micros();

        let paths = Paths::builder().quantity(1).items(path).build()?;

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
