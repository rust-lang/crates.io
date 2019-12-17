use std::error::Error;

use tokio::task::JoinError;

#[derive(Debug)]
pub enum ServiceError {
    JoinError(std::io::Error),
    Hyper(hyper::Error),
}

impl Error for ServiceError {}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::JoinError(e) => e.fmt(f),
            ServiceError::Hyper(e) => e.fmt(f),
        }
    }
}

impl From<JoinError> for ServiceError {
    fn from(e: JoinError) -> Self {
        ServiceError::JoinError(e.into())
    }
}

impl From<hyper::Error> for ServiceError {
    fn from(e: hyper::Error) -> Self {
        ServiceError::Hyper(e)
    }
}
