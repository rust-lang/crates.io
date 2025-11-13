use std::sync::Arc;

use anyhow::Context;
use crates_io_database::models::CloudFrontInvalidationQueueItem;
use crates_io_worker::BackgroundJob;
use serde::{Deserialize, Serialize};

use crate::worker::Environment;
use crate::worker::jobs::ProcessCloudfrontInvalidationQueue;

/// A background job that invalidates the given paths on all CDNs in use on crates.io.
#[derive(Deserialize, Serialize)]
pub struct InvalidateCdns {
    paths: Vec<String>,
}

impl InvalidateCdns {
    pub fn new<I>(paths: I) -> Self
    where
        I: Iterator,
        I::Item: ToString,
    {
        Self {
            paths: paths.map(|path| path.to_string()).collect(),
        }
    }
}

impl BackgroundJob for InvalidateCdns {
    const JOB_NAME: &'static str = "invalidate_cdns";

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        // Fastly doesn't provide an API to purge multiple paths at once, except through the use of
        // surrogate keys. We can't use surrogate keys right now because they require a
        // Fastly-specific header, and not all of our traffic goes through Fastly.
        //
        // For now, we won't parallelise: most crate deletions are for new crates with one (or very
        // few) versions, so the actual number of paths being invalidated is likely to be small, and
        // this is all happening from either a background job or admin command anyway.
        if let Some(fastly) = ctx.fastly()
            && let Some(cdn_domain) = &ctx.config.storage.cdn_prefix
        {
            for path in self.paths.iter() {
                fastly
                    .invalidate(cdn_domain, path)
                    .await
                    .with_context(|| format!("Failed to invalidate path on Fastly CDN: {path}"))?;
            }
        }

        // Queue CloudFront invalidations for batch processing instead of calling directly
        if ctx.cloudfront().is_some() {
            let mut conn = ctx.deadpool.get().await?;

            let result = CloudFrontInvalidationQueueItem::queue_paths(&mut conn, &self.paths).await;
            result.context("Failed to queue CloudFront invalidation paths")?;

            // Schedule the processing job to handle the queued paths
            let result = ProcessCloudfrontInvalidationQueue.enqueue(&mut conn).await;
            result.context("Failed to enqueue CloudFront invalidation processing job")?;
        }

        Ok(())
    }
}
