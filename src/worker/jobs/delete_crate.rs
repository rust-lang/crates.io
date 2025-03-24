use crate::storage::FeedId;
use crate::worker::Environment;
use crate::worker::jobs::InvalidateCdns;
use anyhow::Context;
use crates_io_worker::BackgroundJob;
use std::sync::Arc;
use tokio::try_join;

/// A background job that deletes all files associated with a crate from the storage backend.
#[derive(Serialize, Deserialize)]
pub struct DeleteCrateFromStorage {
    name: String,
}

impl DeleteCrateFromStorage {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl BackgroundJob for DeleteCrateFromStorage {
    const JOB_NAME: &'static str = "delete_crate_from_storage";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let name = &self.name;
        let feed_id = FeedId::Crate { name };

        let (crate_file_paths, readme_paths, _) = try_join!(
            async {
                info!("{name}: Deleting crate files from S3…");
                let result = ctx.storage.delete_all_crate_files(name).await;
                result.context("Failed to delete crate files from S3")
            },
            async {
                info!("{name}: Deleting readme files from S3…");
                let result = ctx.storage.delete_all_readmes(name).await;
                result.context("Failed to delete readme files from S3")
            },
            async {
                info!("{name}: Deleting RSS feed from S3…");
                let result = ctx.storage.delete_feed(&feed_id).await;
                result.context("Failed to delete RSS feed from S3")
            }
        )?;

        info!("{name}: Successfully deleted crate from S3");

        info!("{name}: Enqueuing CDN invalidations");

        let mut conn = ctx.deadpool.get().await?;
        InvalidateCdns::new(
            crate_file_paths
                .into_iter()
                .chain(readme_paths.into_iter())
                .chain(std::iter::once(object_store::path::Path::from(&feed_id))),
        )
        .enqueue(&mut conn)
        .await?;

        info!("{name}: Successfully enqueued CDN invalidations.");

        Ok(())
    }
}
