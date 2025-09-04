use aws_credential_types::Credentials;
use aws_sdk_cloudfront::config::retry::RetryConfig;
use aws_sdk_cloudfront::config::{BehaviorVersion, Region};
use aws_sdk_cloudfront::error::{BuildError, SdkError};
use aws_sdk_cloudfront::operation::create_invalidation::CreateInvalidationError;
use aws_sdk_cloudfront::types::{InvalidationBatch, Paths};
use aws_sdk_cloudfront::{Client, Config};
use tracing::{debug, instrument, warn};

#[derive(Debug, thiserror::Error)]
pub enum CloudFrontError {
    #[error(transparent)]
    BuildError(#[from] BuildError),
    #[error(transparent)]
    SdkError(Box<SdkError<CreateInvalidationError>>),
}

impl From<SdkError<CreateInvalidationError>> for CloudFrontError {
    fn from(err: SdkError<CreateInvalidationError>) -> Self {
        CloudFrontError::SdkError(Box::new(err))
    }
}

impl CloudFrontError {
    pub fn is_too_many_invalidations_error(&self) -> bool {
        let CloudFrontError::SdkError(sdk_error) = self else {
            return false;
        };

        let Some(service_error) = sdk_error.as_service_error() else {
            return false;
        };

        service_error.is_too_many_invalidations_in_progress()
    }
}

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
            .behavior_version(BehaviorVersion::v2025_08_07())
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
    pub async fn invalidate(&self, path: &str) -> Result<(), CloudFrontError> {
        self.invalidate_many(vec![path.to_string()]).await
    }

    /// Invalidate multiple paths on Cloudfront.
    #[instrument(skip(self))]
    pub async fn invalidate_many(&self, mut paths: Vec<String>) -> Result<(), CloudFrontError> {
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

        Ok(invalidation_request
            .send()
            .await
            .map(|_| ()) // We don't care about the result, just that it worked
            .inspect(|_| debug!("Invalidation request successful"))
            .inspect_err(|error| warn!(?error, "Invalidation request failed"))?)
    }
}
