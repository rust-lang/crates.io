use crate::docs_rs::{DocsRsError, docs_rs_client};
use crate::worker::Environment;
use crates_io_worker::BackgroundJob;
use std::sync::Arc;

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
        let client = docs_rs_client(&ctx.config);
        match client.rebuild_docs(&self.name, &self.version).await {
            Ok(()) => Ok(()),
            Err(DocsRsError::BadRequest(msg)) => {
                warn!(
                    name = self.name,
                    version = self.version,
                    "couldn't queue docs rebuild"
                );
                Ok(())
            }
            Err(DocsRsError::RateLimited) => {
                // FIXME: how would we enfore a retry-after?
                Err(err.into())
            }
            Err(err) => Err(err.into()),
        }
    }
}
