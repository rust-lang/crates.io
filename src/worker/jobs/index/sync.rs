use crate::index::get_index_data;
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use crate::worker::jobs::ProcessCloudfrontInvalidationQueue;
use anyhow::Context;
use crates_io_database::models::{CloudFrontInvalidationQueueItem, GitIndexSyncQueueItem};
use crates_io_index::Repository;
use crates_io_worker::BackgroundJob;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, instrument, warn};

/// Maximum number of commits to make within in a single batch.
const BATCH_SIZE: i32 = 100;

#[derive(Serialize, Deserialize)]
pub struct SyncToGitIndex;

#[derive(Debug)]
struct PendingGitIndexSync {
    crate_name: String,
    new: Option<String>,
}

impl SyncToGitIndex {
    /// Processes a single batch of crates that are awaiting Git index updates.
    #[instrument(skip_all)]
    async fn process_batch(
        env: <Self as BackgroundJob>::Context,
        conn: &mut AsyncPgConnection,
    ) -> anyhow::Result<usize> {
        // We'll do this in a transaction so that, on failure, the queue isn't accidentally flushed
        // of crates that weren't actually updated. This should be OK in practice because index file
        // updates are idempotent.
        conn.transaction(|conn| {
            async move {
                // Go get the index data for whatever crates are pending.
                let mut pending = Vec::new();
                for GitIndexSyncQueueItem { crate_name, .. } in
                    GitIndexSyncQueueItem::fetch_batch(conn, BATCH_SIZE)
                        .await?
                        .into_iter()
                {
                    let new = get_index_data(&crate_name, conn, env.config.index_include_pubtime)
                        .await
                        .with_context(|| format!("Failed to get index data for {crate_name}"))?;

                    pending.push(PendingGitIndexSync { crate_name, new });
                }

                // No point continuing (and locking the repo) if there's nothing to do.
                if pending.is_empty() {
                    return Ok(0);
                }

                // Let's go make some commits.
                spawn_blocking(move || Self::commit_and_push_batch(env, pending)).await?
            }
            .scope_boxed()
        })
        .await
    }

    /// Commits updated index files for the given pending crates and pushes them to the origin.
    ///
    /// This will block heavily, and needs to be called on a blocking thread.
    fn commit_and_push_batch(
        env: <Self as BackgroundJob>::Context,
        pending: Vec<PendingGitIndexSync>,
    ) -> anyhow::Result<usize> {
        let repo = env.lock_index()?;
        let num = pending.len();

        let start = Instant::now();
        for PendingGitIndexSync { crate_name, new } in pending.into_iter() {
            let dst = repo.index_file(&crate_name);

            // Read the previous crate contents
            let old = match fs::read_to_string(&dst) {
                Ok(content) => Some(content),
                Err(error) if error.kind() == ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };

            match (old, new) {
                (None, Some(new)) => {
                    fs::create_dir_all(dst.parent().unwrap())?;
                    let mut file = File::create(&dst)?;
                    file.write_all(new.as_bytes())?;
                    repo.commit_file(&format!("Create crate `{crate_name}`"), &dst)?;
                }
                (Some(old), Some(new)) if old != new => {
                    let mut file = File::create(&dst)?;
                    file.write_all(new.as_bytes())?;
                    repo.commit_file(&format!("Update crate `{crate_name}`"), &dst)?;
                }
                (Some(_old), None) => {
                    fs::remove_file(&dst)?;
                    repo.commit_file(&format!("Delete crate `{crate_name}`"), &dst)?;
                }
                _ => {
                    debug!("Skipping sync for {crate_name} because index is up-to-date")
                }
            }
        }

        repo.push()?;
        info!(
            duration = start.elapsed().as_nanos(),
            "Committed and pushed {num} crate update(s)"
        );

        Ok(num)
    }
}

impl BackgroundJob for SyncToGitIndex {
    const JOB_NAME: &'static str = "sync_to_git_index";
    const PRIORITY: i16 = 100;
    const DEDUPLICATED: bool = true;
    const QUEUE: &'static str = "repository";

    type Context = Arc<Environment>;

    /// Regenerates or removes the index files for any crates that are pending.
    #[instrument(skip_all)]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        info!("Syncing to git index");

        let mut conn = env.deadpool.get().await?;
        loop {
            let item_count = Self::process_batch(env.clone(), &mut conn).await?;
            if item_count == 0 {
                return Ok(());
            }
        }
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

        let content = get_index_data(&crate_name, &mut conn, env.config.index_include_pubtime)
            .await
            .context("Failed to get index data")?;

        let future = env.storage.sync_index(&self.krate, content);
        future.await.context("Failed to sync index data")?;

        let path = Repository::relative_index_file_for_url(&self.krate);

        if let Some(fastly) = env.fastly()
            && env.config.sparse_index_fastly_enabled
        {
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

            let paths = &[path];
            let result = CloudFrontInvalidationQueueItem::queue_paths(&mut conn, paths).await;
            result.context("Failed to queue CloudFront invalidation path")?;

            let result = ProcessCloudfrontInvalidationQueue.enqueue(&mut conn).await;
            result.context("Failed to enqueue CloudFront invalidation processing job")?;
        }
        Ok(())
    }
}
