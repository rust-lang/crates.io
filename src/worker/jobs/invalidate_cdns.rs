use std::sync::Arc;

use anyhow::Context;
use crates_io_worker::BackgroundJob;

use crate::worker::Environment;

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
        if let Some(fastly) = ctx.fastly() {
            for path in self.paths.iter() {
                fastly
                    .invalidate(path)
                    .await
                    .with_context(|| format!("Failed to invalidate path on Fastly CDN: {path}"))?;
            }
        }

        if let Some(cloudfront) = ctx.cloudfront() {
            cloudfront
                .invalidate_many(self.paths.clone())
                .await
                .context("Failed to invalidate paths on CloudFront CDN")?;
        }

        Ok(())
    }
}
