use swirl::{Builder, Runner};
use std::panic::AssertUnwindSafe;
use std::sync::{Mutex, MutexGuard};

use crate::db::{DieselPool, DieselPooledConn};
use crate::git::{AddCrate, Repository, Yank};
use crate::util::errors::{CargoResult, CargoErrToStdErr};

impl<'a> swirl::DieselPool<'a> for DieselPool {
    type Connection = DieselPooledConn<'a>;
    type Error = CargoErrToStdErr;

    fn get(&'a self) -> Result<Self::Connection, Self::Error> {
        self.get().map_err(CargoErrToStdErr)
    }
}

pub fn job_runner(config: Builder<Environment, DieselPool>) -> Runner<Environment, DieselPool> {
    config.register::<AddCrate>().register::<Yank>().build()
}

#[allow(missing_debug_implementations)]
pub struct Environment {
    index: Mutex<Repository>,
    pub credentials: Option<(String, String)>,
    // FIXME: https://github.com/sfackler/r2d2/pull/70
    pub connection_pool: AssertUnwindSafe<DieselPool>,
}

impl Environment {
    pub fn new(
        index: Repository,
        credentials: Option<(String, String)>,
        connection_pool: DieselPool,
    ) -> Self {
        Self {
            index: Mutex::new(index),
            credentials,
            connection_pool: AssertUnwindSafe(connection_pool),
        }
    }

    pub fn credentials(&self) -> Option<(&str, &str)> {
        self.credentials
            .as_ref()
            .map(|(u, p)| (u.as_str(), p.as_str()))
    }

    pub fn connection(&self) -> CargoResult<DieselPooledConn> {
        self.connection_pool.0.get().map_err(Into::into)
    }

    pub fn lock_index(&self) -> CargoResult<MutexGuard<'_, Repository>> {
        let repo = self.index.lock().unwrap_or_else(|e| e.into_inner());
        repo.reset_head()?;
        Ok(repo)
    }
}
