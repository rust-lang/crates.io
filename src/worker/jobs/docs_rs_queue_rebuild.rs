use crate::worker::Environment;
use anyhow::anyhow;
use crates_io_docs_rs::{DocsRsClient, DocsRsError, RealDocsRsClient};
use crates_io_worker::BackgroundJob;
use std::sync::Arc;

/// Builds an [DocsRsClient] implementation based on the [crate::config::Server]
pub fn docs_rs_client(
    config: &crate::config::Server,
) -> anyhow::Result<Box<dyn DocsRsClient + Send + Sync>> {
    if let Some(api_token) = &config.docs_rs_api_token {
        Ok(Box::new(RealDocsRsClient::new(
            config.docs_rs_url.clone(),
            api_token,
        )?))
    } else {
        #[cfg(test)]
        {
            use crates_io_docs_rs::MockDocsRsClient;

            Ok(Box::new(MockDocsRsClient::new()))
        }
        #[cfg(not(test))]
        {
            use anyhow::bail;
            bail!("missing docs.rs API token")
        }
    }
}

/// A background job that queues a docs rebuild for a specific release
#[derive(Serialize, Deserialize)]
pub struct DocsRsQueueRebuild {
    name: String,
    version: String,
}

impl DocsRsQueueRebuild {
    pub fn new(name: String, version: String) -> Self {
        Self { name, version }
    }
}

impl BackgroundJob for DocsRsQueueRebuild {
    const JOB_NAME: &'static str = "docs_rs_queue_rebuild";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let client = docs_rs_client(&ctx.config)?;

        match client.rebuild_docs(&self.name, &self.version).await {
            Ok(()) => Ok(()),
            Err(DocsRsError::BadRequest(msg)) => {
                warn!(
                    name = self.name,
                    version = self.version,
                    msg,
                    "couldn't queue docs rebuild"
                );
                Ok(())
            }
            Err(DocsRsError::RateLimited) => {
                Err(anyhow!("docs rebuild request was rate limited. retrying."))
            }
            Err(err) => {
                error!(
                    name = self.name,
                    version = self.version,
                    ?err,
                    "couldn't queue docs rebuild. won't retry"
                );
                Ok(())
            }
        }
    }
}
