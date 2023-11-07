use crate::cloudfront::CloudFront;
use crate::db::DieselPool;
use crate::fastly::Fastly;
use crate::storage::Storage;
use crates_io_index::{Repository, RepositoryConfig};
use parking_lot::{Mutex, MutexGuard};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Instant;

pub struct Environment {
    repository_config: RepositoryConfig,
    repository: Mutex<Option<Repository>>,
    cloudfront: Option<CloudFront>,
    fastly: Option<Fastly>,
    pub storage: Arc<Storage>,
    pub connection_pool: DieselPool,
}

impl Environment {
    pub fn new(
        repository_config: RepositoryConfig,
        cloudfront: Option<CloudFront>,
        fastly: Option<Fastly>,
        storage: Arc<Storage>,
        connection_pool: DieselPool,
    ) -> Self {
        Self {
            repository_config,
            repository: Mutex::new(None),
            cloudfront,
            fastly,
            storage,
            connection_pool,
        }
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
