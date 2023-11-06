use crate::cloudfront::CloudFront;
use crate::fastly::Fastly;
use crate::storage::Storage;
use crate::worker::swirl::PerformError;
use crates_io_index::Repository;
use reqwest::blocking::Client;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

pub struct Environment {
    index: Mutex<Repository>,
    http_client: Client,
    cloudfront: Option<CloudFront>,
    fastly: Option<Fastly>,
    pub storage: Arc<Storage>,
}

impl Environment {
    pub fn new(
        index: Repository,
        http_client: Client,
        cloudfront: Option<CloudFront>,
        fastly: Option<Fastly>,
        storage: Arc<Storage>,
    ) -> Self {
        Self {
            index: Mutex::new(index),
            http_client,
            cloudfront,
            fastly,
            storage,
        }
    }

    #[instrument(skip_all)]
    pub fn lock_index(&self) -> Result<MutexGuard<'_, Repository>, PerformError> {
        let repo = self.index.lock().unwrap_or_else(PoisonError::into_inner);
        repo.reset_head()?;
        Ok(repo)
    }

    /// Returns a client for making HTTP requests to upload crate files.
    pub(crate) fn http_client(&self) -> &Client {
        &self.http_client
    }

    pub(crate) fn cloudfront(&self) -> Option<&CloudFront> {
        self.cloudfront.as_ref()
    }

    pub(crate) fn fastly(&self) -> Option<&Fastly> {
        self.fastly.as_ref()
    }
}
