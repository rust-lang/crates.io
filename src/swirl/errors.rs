use diesel::result::Error as DieselError;
use std::error::Error;
use std::fmt;

use crate::db::PoolError;

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
pub(crate) type PerformError = Box<dyn Error>;

/// An error occurred while attempting to fetch jobs from the queue
pub enum FetchError {
    /// We could not acquire a database connection from the pool.
    ///
    /// Either the connection pool is too small, or new connections cannot be
    /// established.
    NoDatabaseConnection(PoolError),

    /// Could not execute the query to load a job from the database.
    FailedLoadingJob(DieselError),

    /// No message was received from the worker thread.
    ///
    /// Either the thread pool is too small, or jobs have hung indefinitely
    NoMessageReceived,
}

impl fmt::Debug for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::NoDatabaseConnection(e) => {
                f.debug_tuple("NoDatabaseConnection").field(e).finish()
            }
            FetchError::FailedLoadingJob(e) => f.debug_tuple("FailedLoadingJob").field(e).finish(),
            FetchError::NoMessageReceived => f.debug_struct("NoMessageReceived").finish(),
        }
    }
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::NoDatabaseConnection(e) => {
                write!(f, "Timed out acquiring a database connection. ")?;
                write!(f, "Try increasing the connection pool size: ")?;
                write!(f, "{e}")?;
            }
            FetchError::FailedLoadingJob(e) => {
                write!(f, "An error occurred loading a job from the database: ")?;
                write!(f, "{e}")?;
            }
            FetchError::NoMessageReceived => {
                write!(f, "No message was received from the worker thread. ")?;
                write!(f, "Try increasing the thread pool size or timeout period.")?;
            }
        }
        Ok(())
    }
}

impl Error for FetchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FetchError::NoDatabaseConnection(e) => Some(e),
            FetchError::FailedLoadingJob(e) => Some(e),
            FetchError::NoMessageReceived => None,
        }
    }
}

/// An error returned by `Runner::check_for_failed_jobs`. Only used in tests.
#[derive(Debug)]
pub enum FailedJobsError {
    /// Jobs failed to run
    JobsFailed(
        /// The number of failed jobs
        i64,
    ),

    #[doc(hidden)]
    /// Match on `_` instead, more variants may be added in the future
    /// Some other error occurred. Worker threads may have panicked, an error
    /// occurred counting failed jobs in the DB, or something else
    /// unexpectedly went wrong.
    __Unknown(Box<dyn Error + Send + Sync>),
}

pub(super) use FailedJobsError::JobsFailed;

impl From<Box<dyn Error + Send + Sync>> for FailedJobsError {
    fn from(e: Box<dyn Error + Send + Sync>) -> Self {
        FailedJobsError::__Unknown(e)
    }
}

impl From<DieselError> for FailedJobsError {
    fn from(e: DieselError) -> Self {
        FailedJobsError::__Unknown(e.into())
    }
}

impl PartialEq for FailedJobsError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (JobsFailed(x), JobsFailed(y)) => x == y,
            _ => false,
        }
    }
}

impl fmt::Display for FailedJobsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use FailedJobsError::*;

        match self {
            JobsFailed(x) => write!(f, "{x} jobs failed"),
            FailedJobsError::__Unknown(e) => e.fmt(f),
        }
    }
}

impl Error for FailedJobsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            JobsFailed(_) => None,
            FailedJobsError::__Unknown(e) => Some(&**e),
        }
    }
}
