use std::fmt;

use openssl::error::ErrorStack;
use reqwest::Error as ReqwestError;

#[derive(Debug)]
pub enum Error {
    Openssl(ErrorStack),
    Reqwest(ReqwestError),
}

impl From<ErrorStack> for Error {
    fn from(stack: ErrorStack) -> Self {
        Self::Openssl(stack)
    }
}

impl From<ReqwestError> for Error {
    fn from(error: ReqwestError) -> Self {
        Self::Reqwest(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Openssl(stack) => stack.fmt(f),
            Self::Reqwest(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {}
