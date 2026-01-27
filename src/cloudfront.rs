use aws_credential_types::Credentials;
use aws_sdk_cloudfront::config::retry::RetryConfig;
use aws_sdk_cloudfront::config::{BehaviorVersion, Region};
use aws_sdk_cloudfront::error::{BuildError, SdkError};
use aws_sdk_cloudfront::operation::create_invalidation::CreateInvalidationError;
use aws_sdk_cloudfront::types::{InvalidationBatch, Paths};
use aws_sdk_cloudfront::{Client, Config};
use crates_io_database::models::CloudFrontDistribution;
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
    index_distribution_id: String,
    static_distribution_id: String,
}

impl CloudFront {
    pub fn from_environment() -> Option<Self> {
        let access_key = match dotenvy::var("AWS_ACCESS_KEY") {
            Ok(a) => a,
            Err(_) => {
                warn!("Missing AWS_ACCESS_KEY environment variable");
                return None;
            }
        };
        let secret_key = match dotenvy::var("AWS_SECRET_KEY") {
            Ok(s) => s,
            Err(_) => {
                warn!("Missing AWS_SECRET_KEY environment variable");
                return None;
            }
        };

        let index_distribution_id = match dotenvy::var("CLOUDFRONT_DISTRIBUTION_ID_INDEX") {
            Ok(id) => id,
            Err(_) => {
                warn!("Missing CLOUDFRONT_DISTRIBUTION_ID_INDEX environment variable");
                return None;
            }
        };

        let static_distribution_id = match dotenvy::var("CLOUDFRONT_DISTRIBUTION_ID_STATIC") {
            Ok(id) => id,
            Err(_) => {
                warn!("Missing CLOUDFRONT_DISTRIBUTION_ID_STATIC environment variable");
                return None;
            }
        };

        let credentials = Credentials::from_keys(access_key, secret_key, None);

        let config = Config::builder()
            .behavior_version(BehaviorVersion::v2026_01_12())
            .region(Region::new("us-east-1"))
            .credentials_provider(credentials)
            .retry_config(RetryConfig::standard().with_max_attempts(10))
            .build();

        let client = Client::from_conf(config);

        Some(Self {
            client,
            index_distribution_id,
            static_distribution_id,
        })
    }

    /// Returns the distribution ID for the given distribution.
    pub fn distribution_id(&self, distribution: CloudFrontDistribution) -> &str {
        match distribution {
            CloudFrontDistribution::Static => &self.static_distribution_id,
            CloudFrontDistribution::Index => &self.index_distribution_id,
        }
    }

    /// Invalidate multiple paths on CloudFront for a specific distribution.
    #[instrument(skip(self))]
    pub async fn invalidate_many(
        &self,
        distribution: CloudFrontDistribution,
        mut paths: Vec<String>,
    ) -> Result<(), CloudFrontError> {
        let distribution_id = self.distribution_id(distribution);
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
            .distribution_id(distribution_id)
            .invalidation_batch(invalidation_batch);

        debug!(%distribution_id, "Sending invalidation request");

        Ok(invalidation_request
            .send()
            .await
            .map(|_| ()) // We don't care about the result, just that it worked
            .inspect(|_| debug!("Invalidation request successful"))
            .inspect_err(|error| warn!(?error, "Invalidation request failed"))?)
    }
}
