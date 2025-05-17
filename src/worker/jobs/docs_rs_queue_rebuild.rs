use crate::worker::Environment;
use anyhow::anyhow;
use crates_io_docs_rs::DocsRsError;
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
        let Some(docs_rs) = ctx.docs_rs.as_ref() else {
            warn!("docs.rs not configured, skipping rebuild");
            return Ok(());
        };

        match docs_rs.rebuild_docs(&self.name, &self.version).await {
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
