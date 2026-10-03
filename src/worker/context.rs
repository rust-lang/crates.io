use crate::Emails;
use crate::cloudfront::CloudFront;
use crate::config::SharedConfig;
use crate::metrics::WorkerMetrics;
use crate::storage::Storage;
use crate::typosquat;
use crate::worker::jobs::ProcessCloudfrontInvalidationQueue;
use anyhow::Context;
use bon::Builder;
use crates_io_database::models::{CloudFrontDistribution, CloudFrontInvalidationQueueItem};
use crates_io_docs_rs::DocsRsClient;
use crates_io_fastly::Fastly;
use crates_io_github::GitHubClient;
use crates_io_github_app::GitHubApp;
use crates_io_index::{Repository, RepositoryConfig};
use crates_io_og_image::OgImageGenerator;
use crates_io_team_repo::TeamRepo;
use crates_io_worker::BackgroundJob;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::deadpool::Pool;
use object_store::ObjectStore;
use parking_lot::{Mutex, MutexGuard};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::OnceCell;
use tracing::{info, instrument};

/// Components shared across background worker jobs.
#[doc(hidden)]
#[derive(Builder)]
#[builder(
    builder_type(name = WorkerContextBuilder, vis = "pub"),
    state_mod(name = worker_context_builder, vis = "pub"),
    finish_fn(name = build_inner, vis = "")
)]
pub struct WorkerContextInner {
    pub config: Arc<SharedConfig>,

    /// OpenTelemetry metrics recorded by the background worker.
    pub metrics: WorkerMetrics,

    pub repository_config: RepositoryConfig,
    #[builder(skip)]
    repository: Mutex<CachedIndex>,
    cloudfront: Option<CloudFront>,
    fastly: Option<Fastly>,
    pub storage: Arc<Storage>,
    pub downloads_archive_store: Option<Box<dyn ObjectStore>>,
    pub deadpool: Pool<AsyncPgConnection>,
    pub emails: Emails,
    pub team_repo: Box<dyn TeamRepo + Send + Sync>,
    pub index_sync_github_app: Option<Arc<dyn GitHubApp>>,
    pub sync_github_app: Option<Arc<dyn GitHubApp>>,
    pub github: Arc<dyn GitHubClient>,
    pub docs_rs: Option<Box<dyn DocsRsClient>>,
    pub og_image_generator: Option<OgImageGenerator>,

    /// A lazily initialised cache of the most popular crates ready to use in typosquatting checks.
    #[builder(skip)]
    typosquat_cache: OnceCell<Result<typosquat::Cache, typosquat::CacheError>>,
}

/// The worker's local index clone and whether it must fetch before reuse.
#[derive(Default)]
struct CachedIndex {
    repository: Option<Repository>,
    stale: bool,
}

impl WorkerContext {
    /// Locks the cached index and fetches the remote tip before returning it.
    #[instrument(skip_all)]
    pub fn lock_and_refresh_index(&self) -> anyhow::Result<RepositoryLock<'_>> {
        let mut repo = self.lock_index_repository()?;
        repo.refresh()?;
        Ok(repo)
    }

    /// Locks the cached index, fetching only if it has been marked stale.
    #[instrument(skip_all)]
    pub fn lock_index(&self) -> anyhow::Result<RepositoryLock<'_>> {
        let mut repo = self.lock_index_repository()?;
        if repo.index.stale {
            repo.refresh()?;
        }
        Ok(repo)
    }

    /// Acquires the clone lock and initializes the clone on first use.
    fn lock_index_repository(&self) -> anyhow::Result<RepositoryLock<'_>> {
        let lock_start = Instant::now();
        let mut index = self.repository.lock();
        info!(duration = lock_start.elapsed().as_nanos(), "Index locked");

        if index.repository.is_none() {
            info!("Cloning index");
            let clone_start = Instant::now();

            // The worker only ever reads the current tip and commits/pushes on
            // top of it, so a shallow clone is sufficient and avoids fetching
            // the large index history.
            index.repository = Some(Repository::open_shallow(&self.repository_config)?);
            index.stale = false;

            let clone_duration = clone_start.elapsed();
            info!(duration = clone_duration.as_nanos(), "Index cloned");
        }

        Ok(RepositoryLock { index })
    }

    pub(crate) fn cloudfront(&self) -> Option<&CloudFront> {
        self.cloudfront.as_ref()
    }

    pub(crate) fn fastly(&self) -> Option<&Fastly> {
        self.fastly.as_ref()
    }

    /// Invalidates a file in all registered CDNs.
    pub(crate) async fn invalidate_cdns(
        &self,
        conn: &AsyncPgConnection,
        distribution: CloudFrontDistribution,
        path: &str,
    ) -> anyhow::Result<()> {
        // Queue CloudFront invalidations for batch processing instead of calling directly
        if self.cloudfront().is_some() {
            let paths = &[path.to_string()];
            let result =
                CloudFrontInvalidationQueueItem::queue_paths(conn, distribution, paths).await;
            result.context("Failed to queue CloudFront invalidation path")?;

            // Schedule the processing job to handle the queued paths
            let result = ProcessCloudfrontInvalidationQueue.enqueue(conn).await;
            result.context("Failed to enqueue CloudFront invalidation processing job")?;
        }

        if let Some(fastly) = self.fastly()
            && let Some(cdn_domain) = &self.config.storage.cdn_prefix
        {
            fastly
                .purge_both_domains(cdn_domain, path)
                .await
                .context("Fastly")?;
        }

        Ok(())
    }

    /// Returns the typosquatting cache, initialising it if required.
    pub(crate) async fn typosquat_cache(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> Result<&typosquat::Cache, typosquat::CacheError> {
        // We have to pass conn back in here because the caller might be in a transaction, and
        // getting a new connection here to query crates can result in a deadlock.
        //
        // Note that this intentionally won't retry if the initial call to `from_env` fails:
        // typosquatting checks aren't on the critical path for publishing, and a warning will be
        // generated if initialising the cache fails.
        self.typosquat_cache
            .get_or_init(|| typosquat::Cache::from_env(conn))
            .await
            .as_ref()
            .map_err(|e| e.clone())
    }
}

/// Components shared across background worker jobs.
#[derive(Clone, derive_more::Deref)]
pub struct WorkerContext(Arc<WorkerContextInner>);

impl WorkerContext {
    /// Creates a builder for a worker context.
    pub fn builder() -> WorkerContextBuilder {
        WorkerContextInner::builder()
    }
}

impl<S: worker_context_builder::State> WorkerContextBuilder<S> {
    /// Finishes building the worker context.
    pub fn build(self) -> WorkerContext
    where
        S: worker_context_builder::IsComplete,
    {
        WorkerContext(Arc::new(self.build_inner()))
    }
}

pub struct RepositoryLock<'a> {
    index: MutexGuard<'a, CachedIndex>,
}

impl RepositoryLock<'_> {
    /// Marks the clone for refresh when next reused.
    pub fn mark_stale(&mut self) {
        self.index.stale = true;
    }

    /// Refreshes the clone and retains the stale mark if fetching fails.
    fn refresh(&mut self) -> anyhow::Result<()> {
        self.mark_stale();
        self.reset_head()?;
        self.index.stale = false;
        Ok(())
    }
}

impl Deref for RepositoryLock<'_> {
    type Target = Repository;

    fn deref(&self) -> &Self::Target {
        self.index.repository.as_ref().unwrap()
    }
}

impl DerefMut for RepositoryLock<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.index.repository.as_mut().unwrap()
    }
}
