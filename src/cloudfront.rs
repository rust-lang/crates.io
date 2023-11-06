use aws_credential_types::Credentials;
use aws_sdk_cloudfront::config::Region;
use aws_sdk_cloudfront::error::SdkError;
use aws_sdk_cloudfront::types::{InvalidationBatch, Paths};
use aws_sdk_cloudfront::{Client, Config};
use retry::delay::{jitter, Exponential};
use retry::OperationResult;
use std::time::Duration;
use tokio::runtime::Runtime;

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
            .region(Region::new("us-east-1"))
            .credentials_provider(credentials)
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
    #[instrument(skip(self, rt))]
    pub fn invalidate(&self, path: &str, rt: &Runtime) -> anyhow::Result<()> {
        let path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };

        let attempts = 10;
        let backoff = Exponential::from_millis(500)
            .map(|duration| duration.clamp(Duration::ZERO, Duration::from_secs(30)))
            .map(jitter)
            .take(attempts - 1);

        retry::retry_with_index(backoff, |attempt| {
            let now = chrono::offset::Utc::now().timestamp_micros();

            let paths = match Paths::builder().quantity(1).items(&path).build() {
                Ok(paths) => paths,
                Err(error) => return OperationResult::Err(error.into()),
            };

            let invalidation_batch = match InvalidationBatch::builder()
                .caller_reference(format!("{now}"))
                .paths(paths)
                .build()
            {
                Ok(invalidation_batch) => invalidation_batch,
                Err(error) => return OperationResult::Err(error.into()),
            };

            let invalidation_request = self
                .client
                .create_invalidation()
                .distribution_id(&self.distribution_id)
                .invalidation_batch(invalidation_batch);

            debug!(?attempt, "Sending invalidation request");

            match rt.block_on(invalidation_request.send()) {
                Ok(_) => {
                    debug!("Invalidation request successful");
                    OperationResult::Ok(())
                }
                Err(SdkError::ServiceError(error))
                    if error.err().is_too_many_invalidations_in_progress() =>
                {
                    warn!("Invalidation request failed (TooManyInvalidationsInProgress)");
                    OperationResult::Retry(SdkError::ServiceError(error).into())
                }
                Err(error) => {
                    warn!(?error, "Invalidation request failed");
                    OperationResult::Err(error.into())
                }
            }
        })
        .map_err(|error| error.error)
    }
}
