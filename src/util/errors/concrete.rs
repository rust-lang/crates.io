use std::{error, fmt, io};

#[derive(Debug)]
pub enum Error {
    DbConnect(diesel::result::ConnectionError),
    DbQuery(diesel::result::Error),
    DotEnv(dotenv::Error),
    Internal(String),
    Io(io::Error),
    JobEnqueue(swirl::EnqueueError),
    Openssl(openssl::error::ErrorStack),
    Reqwest(reqwest::Error),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::DbConnect(inner) => inner.fmt(f),
            Error::DbQuery(inner) => inner.fmt(f),
            Error::DotEnv(inner) => inner.fmt(f),
            Error::Internal(inner) => inner.fmt(f),
            Error::Io(inner) => inner.fmt(f),
            Error::JobEnqueue(inner) => inner.fmt(f),
            Error::Openssl(inner) => inner.fmt(f),
            Error::Reqwest(inner) => inner.fmt(f),
        }
    }
}

impl From<diesel::result::ConnectionError> for Error {
    fn from(err: diesel::result::ConnectionError) -> Self {
        Error::DbConnect(err)
    }
}

impl From<diesel::result::Error> for Error {
    fn from(err: diesel::result::Error) -> Self {
        Error::DbQuery(err)
    }
}

impl From<dotenv::Error> for Error {
    fn from(err: dotenv::Error) -> Self {
        Error::DotEnv(err)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::Internal(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<swirl::EnqueueError> for Error {
    fn from(err: swirl::EnqueueError) -> Self {
        Error::JobEnqueue(err)
    }
}

impl From<s3::Error> for Error {
    fn from(err: s3::Error) -> Self {
        match err {
            s3::Error::Openssl(e) => Error::Openssl(e),
            s3::Error::Reqwest(e) => Error::Reqwest(e),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Reqwest(err)
    }
}
