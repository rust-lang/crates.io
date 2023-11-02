use crate::db::PoolError;
use diesel::result::Error as DieselError;
use std::error::Error;

/// An error occurred queueing the job
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EnqueueError {
    /// An error occurred serializing the job
    #[error(transparent)]
    SerializationError(#[from] serde_json::error::Error),

    /// An error occurred inserting the job into the database
    #[error(transparent)]
    DatabaseError(#[from] DieselError),
}

/// An error occurred performing the job
pub type PerformError = Box<dyn Error>;

/// An error occurred while attempting to fetch jobs from the queue
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    /// We could not acquire a database connection from the pool.
    ///
    /// Either the connection pool is too small, or new connections cannot be
    /// established.
    #[error("Timed out acquiring a database connection. Try increasing the connection pool size.")]
    NoDatabaseConnection(#[source] PoolError),

    /// Could not execute the query to load a job from the database.
    #[error("An error occurred loading a job from the database.")]
    FailedLoadingJob(#[source] DieselError),

    /// No message was received from the worker thread.
    ///
    /// Either the thread pool is too small, or jobs have hung indefinitely
    #[error("No message was received from the worker thread. Try increasing the thread pool size or timeout period.")]
    NoMessageReceived,
}

/// An error returned by `Runner::check_for_failed_jobs`. Only used in tests.
#[derive(Debug, thiserror::Error)]
pub enum FailedJobsError {
    /// Jobs failed to run
    #[error("{0} jobs failed")]
    JobsFailed(
        /// The number of failed jobs
        i64,
    ),

    #[doc(hidden)]
    /// Match on `_` instead, more variants may be added in the future
    /// Some other error occurred. Worker threads may have panicked, an error
    /// occurred counting failed jobs in the DB, or something else
    /// unexpectedly went wrong.
    #[error(transparent)]
    __Unknown(#[from] Box<dyn Error + Send + Sync>),
}

impl From<DieselError> for FailedJobsError {
    fn from(e: DieselError) -> Self {
        FailedJobsError::__Unknown(e.into())
    }
}

impl PartialEq for FailedJobsError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FailedJobsError::JobsFailed(x), FailedJobsError::JobsFailed(y)) => x == y,
            _ => false,
        }
    }
}
