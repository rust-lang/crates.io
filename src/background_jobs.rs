use reqwest::blocking::Client;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use swirl::PerformError;

use crate::db::{DieselPool, DieselPooledConn, PoolError};
use crate::git::Repository;
use crate::uploaders::Uploader;

impl<'a> swirl::db::BorrowedConnection<'a> for DieselPool {
    type Connection = DieselPooledConn<'a>;
}

impl swirl::db::DieselPool for DieselPool {
    type Error = PoolError;

    fn get(&self) -> Result<swirl::db::DieselPooledConn<'_, Self>, Self::Error> {
        self.get()
    }
}

pub struct Environment {
    index: Arc<Mutex<Repository>>,
    pub uploader: Uploader,
    http_client: AssertUnwindSafe<Client>,
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        Self {
            index: self.index.clone(),
            uploader: self.uploader.clone(),
            http_client: AssertUnwindSafe(self.http_client.0.clone()),
        }
    }
}

impl Environment {
    pub fn new(index: Repository, uploader: Uploader, http_client: Client) -> Self {
        Self::new_shared(Arc::new(Mutex::new(index)), uploader, http_client)
    }

    pub fn new_shared(
        index: Arc<Mutex<Repository>>,
        uploader: Uploader,
        http_client: Client,
    ) -> Self {
        Self {
            index,
            uploader,
            http_client: AssertUnwindSafe(http_client),
        }
    }

    pub fn lock_index(&self) -> Result<MutexGuard<'_, Repository>, PerformError> {
        let repo = self.index.lock().unwrap_or_else(PoisonError::into_inner);
        repo.reset_head()?;
        Ok(repo)
    }

    /// Returns a client for making HTTP requests to upload crate files.
    pub(crate) fn http_client(&self) -> &Client {
        &self.http_client
    }
}
