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
