use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    DbConnect(diesel::result::ConnectionError),
    Internal(String),
    JobEnqueue(swirl::EnqueueError),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DbConnect(inner) => inner.fmt(f),
            Error::Internal(inner) => inner.fmt(f),
            Error::JobEnqueue(inner) => inner.fmt(f),
        }
    }
}

impl From<diesel::result::ConnectionError> for Error {
    fn from(err: diesel::result::ConnectionError) -> Self {
        Error::DbConnect(err)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::Internal(err)
    }
}

impl From<swirl::EnqueueError> for Error {
    fn from(err: swirl::EnqueueError) -> Self {
        Error::JobEnqueue(err)
    }
}
