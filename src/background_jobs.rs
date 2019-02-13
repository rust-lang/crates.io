use url::Url;
use std::panic::AssertUnwindSafe;

use crate::background::{Builder, Runner};
use crate::db::{DieselPool, DieselPooledConn};
use crate::git::{AddCrate, Yank};
use crate::util::CargoResult;

pub fn job_runner(config: Builder<Environment>) -> Runner<Environment> {
    config.register::<AddCrate>().register::<Yank>().build()
}

#[allow(missing_debug_implementations)]
pub struct Environment {
    pub index_location: Url,
    pub credentials: Option<(String, String)>,
    // FIXME: https://github.com/sfackler/r2d2/pull/70
    pub connection_pool: AssertUnwindSafe<DieselPool>,
}

impl Environment {
    pub fn new(
        index_location: Url,
        credentials: Option<(String, String)>,
        connection_pool: DieselPool,
    ) -> Self {
        Self {
            index_location,
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
}
