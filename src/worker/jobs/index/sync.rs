use crate::index::get_index_data;
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use crate::worker::jobs::ProcessCloudfrontInvalidationQueue;
use anyhow::Context;
use crates_io_database::models::{CloudFrontDistribution, CloudFrontInvalidationQueueItem};
use crates_io_index::Repository;
use crates_io_worker::BackgroundJob;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::sync::Arc;
use std::time::Instant;
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

        let new = get_index_data(&crate_name, &mut conn, env.config.index_include_pubtime)
            .await
            .context("Failed to get index data")?;

        spawn_blocking(move || {
            let repo = env.lock_index()?;
            let dst = repo.index_file(&crate_name);

            // Read the previous crate contents
            let old = match fs::read_to_string(&dst) {
                Ok(content) => Some(content),
                Err(error) if error.kind() == ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };

            let commit_and_push_start = Instant::now();
            match (old, new) {
                (None, Some(new)) => {
                    fs::create_dir_all(dst.parent().unwrap())?;
                    let mut file = File::create(&dst)?;
                    file.write_all(new.as_bytes())?;
                    repo.commit_and_push(&format!("Create crate `{}`", &crate_name), &[&dst])?;
                }
                (Some(old), Some(new)) if old != new => {
                    let mut file = File::create(&dst)?;
                    file.write_all(new.as_bytes())?;
                    repo.commit_and_push(&format!("Update crate `{}`", &crate_name), &[&dst])?;
                }
                (Some(_old), None) => {
                    fs::remove_file(&dst)?;
                    repo.commit_and_push(&format!("Delete crate `{}`", &crate_name), &[&dst])?;
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

            let dist = CloudFrontDistribution::Index;
            let paths = &[path];
            let result = CloudFrontInvalidationQueueItem::queue_paths(&mut conn, dist, paths).await;
            result.context("Failed to queue CloudFront invalidation path")?;

            let result = ProcessCloudfrontInvalidationQueue.enqueue(&mut conn).await;
            result.context("Failed to enqueue CloudFront invalidation processing job")?;
        }
        Ok(())
    }
}
