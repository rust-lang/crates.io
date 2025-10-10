use crate::cloudfront::{CloudFront, CloudFrontError};
use crate::worker::Environment;
use anyhow::Context;
use crates_io_database::models::CloudFrontInvalidationQueueItem;
use crates_io_worker::BackgroundJob;
use diesel_async::AsyncPgConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{info, instrument, warn};

/// Maximum number of paths to process in a single batch.
/// Conservative limit to stay within AWS CloudFront's 3,000 path limit per invalidation.
const BATCH_SIZE: usize = 1000;

const INITIAL_BACKOFF: Duration = Duration::from_secs(30);
const MAX_BACKOFF: Duration = Duration::from_secs(15 * 60);
const MAX_RETRIES: u32 = 6; // 30s, 1m, 2m, 4m, 8m, 15m

/// Background job that processes CloudFront invalidation paths from the database queue in batches.
///
/// This job:
/// - Processes up to 1,000 paths per batch to stay within AWS limits
/// - Deduplicates paths before sending to CloudFront
/// - Implements exponential backoff for `TooManyInvalidationsInProgress` errors
/// - Processes all available batches in a single job run
#[derive(Deserialize, Serialize)]
pub struct ProcessCloudfrontInvalidationQueue;

impl ProcessCloudfrontInvalidationQueue {
    #[instrument(skip_all)]
    async fn process_batch(
        &self,
        conn: &mut AsyncPgConnection,
        cloudfront: &CloudFront,
    ) -> anyhow::Result<usize> {
        let items = CloudFrontInvalidationQueueItem::fetch_batch(conn, BATCH_SIZE as i64).await?;
        if items.is_empty() {
            info!("No more CloudFront invalidations to process");
            return Ok(0);
        }

        let item_count = items.len();
        info!("Processing next {item_count} CloudFront invalidations…");

        let mut unique_paths = HashSet::with_capacity(item_count);
        let mut item_ids = Vec::with_capacity(item_count);
        for item in items {
            unique_paths.insert(item.path);
            item_ids.push(item.id);
        }
        let unique_paths: Vec<String> = unique_paths.into_iter().collect();

        let result = self.invalidate_with_backoff(cloudfront, unique_paths).await;
        result.context("Failed to request CloudFront invalidations")?;

        let result = CloudFrontInvalidationQueueItem::remove_items(conn, &item_ids).await;
        result.context("Failed to remove CloudFront invalidations from the queue")?;

        info!("Successfully processed {item_count} CloudFront invalidations");

        Ok(item_count)
    }

    /// Invalidate paths on CloudFront with exponential backoff for `TooManyInvalidationsInProgress`
    #[instrument(skip_all)]
    async fn invalidate_with_backoff(
        &self,
        cloudfront: &CloudFront,
        paths: Vec<String>,
    ) -> Result<(), CloudFrontError> {
        let mut attempt = 1;
        let mut backoff = INITIAL_BACKOFF;
        loop {
            let Err(error) = cloudfront.invalidate_many(paths.clone()).await else {
                return Ok(());
            };

            if !error.is_too_many_invalidations_error() || attempt >= MAX_RETRIES {
                return Err(error);
            }

            warn!(
                "Too many CloudFront invalidations in progress, retrying in {backoff:?} seconds…",
            );

            tokio::time::sleep(backoff).await;

            attempt += 1;
            backoff = std::cmp::min(backoff * 2, MAX_BACKOFF);
        }
    }
}

impl BackgroundJob for ProcessCloudfrontInvalidationQueue {
    const JOB_NAME: &'static str = "process_cloudfront_invalidation_queue";
    const DEDUPLICATED: bool = true;
    const QUEUE: &'static str = "cloudfront";

    type Context = Arc<Environment>;

    #[instrument(skip_all)]
    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let Some(cloudfront) = ctx.cloudfront() else {
            warn!("CloudFront not configured, skipping queue processing");
            return Ok(());
        };

        let mut conn = ctx.deadpool.get().await?;

        // Process batches until the queue is empty, or we hit an error
        loop {
            let item_count = self.process_batch(&mut conn, cloudfront).await?;
            if item_count == 0 {
                // Queue is empty, we're done
                break;
            }
        }

        Ok(())
    }
}
