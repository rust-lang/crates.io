use crate::cloudfront::CloudFront;
use crate::fastly::Fastly;
use crate::storage::Storage;
use crate::typosquat;
use crate::util::diesel::Conn;
use crate::Emails;
use anyhow::Context;
use crates_io_index::{Repository, RepositoryConfig};
use crates_io_team_repo::TeamRepo;
use derive_builder::Builder;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::AsyncPgConnection;
use object_store::ObjectStore;
use parking_lot::{Mutex, MutexGuard};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct Environment {
    pub config: Arc<crate::config::Server>,

    repository_config: RepositoryConfig,
    #[builder(default, setter(skip))]
    repository: Mutex<Option<Repository>>,
    #[builder(default)]
    cloudfront: Option<CloudFront>,
    #[builder(default)]
    fastly: Option<Fastly>,
    pub storage: Arc<Storage>,
    #[builder(default)]
    pub downloads_archive_store: Option<Box<dyn ObjectStore>>,
    pub deadpool: Pool<AsyncPgConnection>,
    pub emails: Emails,
    pub team_repo: Box<dyn TeamRepo + Send + Sync>,

    /// A lazily initialised cache of the most popular crates ready to use in typosquatting checks.
    #[builder(default, setter(skip))]
    typosquat_cache: OnceLock<Result<typosquat::Cache, typosquat::CacheError>>,
}

impl Environment {
    pub fn builder() -> EnvironmentBuilder {
        EnvironmentBuilder::default()
    }

    #[instrument(skip_all)]
    pub fn lock_index(&self) -> anyhow::Result<RepositoryLock<'_>> {
        let mut repo = self.repository.lock();

        if repo.is_none() {
            info!("Cloning index");
            let clone_start = Instant::now();

            *repo = Some(Repository::open(&self.repository_config)?);

            let clone_duration = clone_start.elapsed();
            info!(duration = ?clone_duration, "Index cloned");
        }

        let repo_lock = RepositoryLock { repo };
        repo_lock.reset_head()?;
        Ok(repo_lock)
    }

    pub(crate) fn cloudfront(&self) -> Option<&CloudFront> {
        self.cloudfront.as_ref()
    }

    pub(crate) fn fastly(&self) -> Option<&Fastly> {
        self.fastly.as_ref()
    }

    /// Invalidate a file in all registered CDNs.
    pub(crate) async fn invalidate_cdns(&self, path: &str) -> anyhow::Result<()> {
        if let Some(cloudfront) = self.cloudfront() {
            cloudfront.invalidate(path).await.context("CloudFront")?;
        }

        if let Some(fastly) = self.fastly() {
            fastly.invalidate(path).await.context("Fastly")?;
        }

        Ok(())
    }

    /// Returns the typosquatting cache, initialising it if required.
    pub(crate) fn typosquat_cache(
        &self,
        conn: &mut impl Conn,
    ) -> Result<&typosquat::Cache, typosquat::CacheError> {
        // We have to pass conn back in here because the caller might be in a transaction, and
        // getting a new connection here to query crates can result in a deadlock.
        //
        // Note that this intentionally won't retry if the initial call to `from_env` fails:
        // typosquatting checks aren't on the critical path for publishing, and a warning will be
        // generated if initialising the cache fails.
        self.typosquat_cache
            .get_or_init(|| typosquat::Cache::from_env(conn))
            .as_ref()
            .map_err(|e| e.clone())
    }
}

pub struct RepositoryLock<'a> {
    repo: MutexGuard<'a, Option<Repository>>,
}

impl Deref for RepositoryLock<'_> {
    type Target = Repository;

    fn deref(&self) -> &Self::Target {
        self.repo.as_ref().unwrap()
    }
}

impl DerefMut for RepositoryLock<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.repo.as_mut().unwrap()
    }
}
