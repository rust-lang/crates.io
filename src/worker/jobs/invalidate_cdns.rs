use std::sync::Arc;

use anyhow::Context;
use crates_io_database::models::{CloudFrontDistribution, CloudFrontInvalidationQueueItem};
use crates_io_worker::BackgroundJob;
use serde::{Deserialize, Serialize};

use crate::worker::Environment;
use crate::worker::jobs::ProcessCloudfrontInvalidationQueue;

/// A background job that invalidates the given paths or cache tags on all CDNs used by crates.io.
#[derive(Deserialize, Serialize)]
pub struct InvalidateCdns {
    paths: Vec<String>,

    #[serde(default)]
    cache_tags: Vec<String>,
}

impl InvalidateCdns {
    pub fn paths<I>(paths: I) -> Self
    where
        I: Iterator,
        I::Item: ToString,
    {
        Self {
            paths: paths.map(|path| path.to_string()).collect(),
            cache_tags: Vec::new(),
        }
    }

    pub fn cache_tags<I>(cache_tags: I) -> Self
    where
        I: IntoIterator,
        I::Item: ToString,
    {
        Self {
            paths: Vec::new(),
            cache_tags: cache_tags
                .into_iter()
                .map(|cache_tag| cache_tag.to_string())
                .collect(),
        }
    }

    fn cloudfront_items(&self) -> Vec<String> {
        self.paths
            .iter()
            .cloned()
            .chain(
                self.cache_tags
                    .iter()
                    .map(|cache_tag| format!("#{cache_tag}")),
            )
            .collect()
    }
}

impl BackgroundJob for InvalidateCdns {
    const JOB_NAME: &'static str = "invalidate_cdns";

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        // We won't parallelise: most crate deletions are for new crates with one (or very few)
        // versions, so the number of invalidations is likely to be small.
        if let Some(fastly) = ctx.fastly()
            && let Some(cdn_domain) = &ctx.config.storage.cdn_prefix
        {
            for path in self.paths.iter() {
                fastly
                    .purge_both_domains(cdn_domain, path)
                    .await
                    .with_context(|| format!("Failed to invalidate path on Fastly CDN: {path}"))?;
            }
        }

        if let Some(fastly) = ctx.fastly()
            && !self.cache_tags.is_empty()
        {
            let config = ctx.config.fastly.as_ref();
            let config = config.context("Fastly configuration is missing")?;
            let service_id = config
                .service_id_static
                .as_deref()
                .context("FASTLY_SERVICE_ID_STATIC is not configured")?;

            for cache_tag in &self.cache_tags {
                fastly
                    .purge_surrogate_key(service_id, cache_tag)
                    .await
                    .with_context(|| {
                        format!("Failed to invalidate cache tag on Fastly CDN: {cache_tag}")
                    })?;
            }
        }

        // Queue CloudFront invalidations for batch processing instead of calling directly
        if ctx.cloudfront().is_some() {
            let conn = ctx.deadpool.get().await?;
            let items = self.cloudfront_items();

            let dist = CloudFrontDistribution::Static;
            let result = CloudFrontInvalidationQueueItem::queue_paths(&conn, dist, &items).await;
            result.context("Failed to queue CloudFront invalidations")?;

            // Schedule the processing job to handle the queued paths
            let result = ProcessCloudfrontInvalidationQueue.enqueue(&conn).await;
            result.context("Failed to enqueue CloudFront invalidation processing job")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloudfront_items_include_paths_before_cache_tags() {
        let job: InvalidateCdns = serde_json::from_value(serde_json::json!({
            "paths": ["/path"],
            "cache_tags": ["crate:foo"],
        }))
        .unwrap();

        assert_eq!(job.cloudfront_items(), ["/path", "#crate:foo"]);
    }
}
