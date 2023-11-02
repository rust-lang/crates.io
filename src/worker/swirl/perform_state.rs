use crate::db::ConnectionPool;
use crate::worker::swirl::PerformError;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::PgConnection;

/// Database state that is passed to `Job::perform()`.
pub struct PerformState<'a> {
    /// The existing connection used to lock the background job.
    ///
    /// Most jobs can reuse the existing connection, however it will already be within a
    /// transaction and is thus not appropriate in all cases.
    pub(crate) conn: &'a mut PgConnection,
    /// A connection pool for obtaining a unique connection.
    ///
    /// This will be `None` within our standard test framework, as there everything is expected to
    /// run within a single transaction.
    pub(crate) pool: Option<ConnectionPool>,
}

impl PerformState<'_> {
    /// A helper function for jobs needing a fresh connection (i.e. not already within a transaction).
    ///
    /// This will error when run from our main test framework, as there most work is expected to be
    /// done within an existing transaction.
    pub fn fresh_connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, PerformError> {
        match self.pool {
            // In production a pool should be available. This can only be hit in tests, which don't
            // provide the pool.
            None => Err(String::from("Database pool was unavailable").into()),
            Some(ref pool) => Ok(pool.get()?),
        }
    }
}
