use crate::storage::{StorageKey, crate_cache_tag};
use crate::worker::Environment;
use crate::worker::jobs::InvalidateCdns;
use anyhow::Context;
use crates_io_worker::BackgroundJob;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::try_join;
use tracing::info;

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

    async fn run(self, ctx: Self::Context) -> anyhow::Result<()> {
        let name = &self.name;
        let og_image_key = StorageKey::for_og_image(name);
        let feed_key = StorageKey::CrateFeed { name };

        try_join!(
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
                let result = ctx.storage.delete(&feed_key).await;
                result.context("Failed to delete RSS feed from S3")
            },
            async {
                info!("{name}: Deleting OG image from S3…");
                let result = ctx.storage.delete(&og_image_key).await;
                result.context("Failed to delete OG image from S3")
            }
        )?;

        info!("{name}: Successfully deleted crate from S3");

        info!("{name}: Enqueuing CDN invalidations");

        let conn = ctx.deadpool.get().await?;
        let job = InvalidateCdns::cache_tags([crate_cache_tag(name)]);
        job.enqueue(&conn).await?;

        info!("{name}: Successfully enqueued CDN invalidations.");

        Ok(())
    }
}
