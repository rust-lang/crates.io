use crate::index::get_index_data;
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use crate::worker::jobs::ProcessCloudfrontInvalidationQueue;
use anyhow::Context;
use crates_io_database::models::{CloudFrontDistribution, CloudFrontInvalidationQueueItem};
use crates_io_index::Repository;
use crates_io_worker::BackgroundJob;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::runtime::Handle;
use tracing::{debug, info, instrument, warn};

#[derive(Serialize, Deserialize)]
pub struct SyncToGitIndex {
    krate: String,
}

impl SyncToGitIndex {
    pub fn new(krate: impl Into<String>) -> Self {
        let krate = krate.into();
        Self { krate }
    }
}

impl BackgroundJob for SyncToGitIndex {
    const JOB_NAME: &'static str = "sync_to_git_index";
    const PRIORITY: i16 = 100;
    const DEDUPLICATED: bool = true;
    const QUEUE: &'static str = "repository";

    type Context = Arc<Environment>;

    /// Regenerates or removes an index file for a single crate
    #[instrument(skip_all, fields(krate.name = self.krate))]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        info!("Syncing to git index");

        let crate_name = self.krate.clone();
        let mut conn = env.deadpool.get().await?;

        let new = get_index_data(
            &crate_name,
            &mut conn,
            env.config.features.index_include_pubtime,
        )
        .await
        .context("Failed to get index data")?;

        spawn_blocking(move || {
            let repo = env.lock_index()?;
            let old = repo.read_entry(&crate_name)?;

            let commit_and_push_start = Instant::now();
            match (old, new) {
                (None, Some(new)) => {
                    let msg = format!("Create crate `{crate_name}`");
                    let mut builder = repo.commit_builder(msg)?;
                    builder.upsert_entry(&crate_name, new.as_bytes())?;
                    builder.commit_and_push()?;
                }
                (Some(old), Some(new)) if old != new.as_bytes() => {
                    let msg = format!("Update crate `{crate_name}`");
                    let mut builder = repo.commit_builder(msg)?;
                    builder.upsert_entry(&crate_name, new.as_bytes())?;
                    builder.commit_and_push()?;
                }
                (Some(_old), None) => {
                    let msg = format!("Delete crate `{crate_name}`");
                    let mut builder = repo.commit_builder(msg)?;
                    builder.remove_entry(&crate_name)?;
                    builder.commit_and_push()?;
                }
                _ => debug!("Skipping sync because index is up-to-date"),
            }
            info!(
                duration = commit_and_push_start.elapsed().as_nanos(),
                "Committed and pushed"
            );

            Ok(())
        })
        .await?
    }
}

/// Syncs index files for multiple crates in a single commit
#[derive(Serialize, Deserialize)]
pub struct BulkSyncToGitIndex {
    crate_names: Vec<String>,
    commit_message: String,
}

impl BulkSyncToGitIndex {
    pub fn new(crate_names: Vec<String>, commit_message: impl Into<String>) -> Self {
        Self {
            crate_names,
            commit_message: commit_message.into(),
        }
    }
}

impl BackgroundJob for BulkSyncToGitIndex {
    const JOB_NAME: &'static str = "bulk_sync_to_git_index";
    const QUEUE: &'static str = "repository";

    type Context = Arc<Environment>;

    #[instrument(skip_all, fields(num_crates = self.crate_names.len()))]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        info!(commit_message = ?self.commit_message, "Syncing {} crates to git index", self.crate_names.len());

        let crate_names = self.crate_names.clone();
        let commit_message = self.commit_message.clone();

        let handle = Handle::current();
        spawn_blocking(move || {
            let repo = env.lock_index()?;
            let include_pubtime = env.config.features.index_include_pubtime;

            let mut builder = repo.commit_builder(commit_message)?;
            let mut num_changes = 0;

            for crate_name in &crate_names {
                // Fetch index data using async database queries
                let new = handle
                    .block_on(async {
                        let mut conn = env.deadpool.get().await?;
                        get_index_data(crate_name, &mut conn, include_pubtime).await
                    })
                    .with_context(|| format!("Failed to get index data for `{crate_name}`"))?;

                let old = repo.read_entry(crate_name)?;

                match (old, new) {
                    (None, Some(new)) => {
                        builder.upsert_entry(crate_name, new.as_bytes())?;
                        num_changes += 1;
                    }
                    (Some(old), Some(new)) if old != new.as_bytes() => {
                        builder.upsert_entry(crate_name, new.as_bytes())?;
                        num_changes += 1;
                    }
                    (Some(_old), None) => {
                        builder.remove_entry(crate_name)?;
                        num_changes += 1;
                    }
                    _ => debug!(%crate_name, "Skipping sync because index is up-to-date"),
                }
            }

            if num_changes == 0 {
                info!("No changes to commit");
                return Ok(());
            }

            info!("Committing {num_changes} modified files");
            builder.commit_and_push()?;

            Ok(())
        })
        .await?
    }
}

#[derive(Serialize, Deserialize)]
pub struct SyncToSparseIndex {
    krate: String,
}

impl SyncToSparseIndex {
    pub fn new(krate: impl Into<String>) -> Self {
        let krate = krate.into();
        Self { krate }
    }
}

impl BackgroundJob for SyncToSparseIndex {
    const JOB_NAME: &'static str = "sync_to_sparse_index";
    const PRIORITY: i16 = 100;
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    /// Regenerates or removes an index file for a single crate
    #[instrument(skip_all, fields(krate.name = self.krate))]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        info!("Syncing to sparse index");

        let crate_name = self.krate.clone();
        let mut conn = env.deadpool.get().await?;

        let content = get_index_data(
            &crate_name,
            &mut conn,
            env.config.features.index_include_pubtime,
        )
        .await
        .context("Failed to get index data")?;

        let future = env.storage.sync_index(&self.krate, content);
        future.await.context("Failed to sync index data")?;

        let path = Repository::relative_index_file_for_url(&self.krate);

        if let Some(fastly) = env.fastly() {
            let domain_name = &env.config.domain_name;
            let domains = [
                format!("index.{}", domain_name),
                format!("fastly-index.{}", domain_name),
            ];

            for domain in domains {
                if let Err(error) = fastly.purge(&domain, &path).await {
                    warn!(
                        domain,
                        path, "Failed to invalidate sparse index on Fastly: {error}"
                    );
                }
            }
        }

        if env.cloudfront().is_some() {
            info!(%path, "Queuing index file invalidation on CloudFront");

            let dist = CloudFrontDistribution::Index;
            let paths = &[path];
            let result = CloudFrontInvalidationQueueItem::queue_paths(&conn, dist, paths).await;
            result.context("Failed to queue CloudFront invalidation path")?;

            let result = ProcessCloudfrontInvalidationQueue.enqueue(&conn).await;
            result.context("Failed to enqueue CloudFront invalidation processing job")?;
        }
        Ok(())
    }
}
