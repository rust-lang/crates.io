use std::fmt;

use super::{json_error, AppError};
use crate::util::AppResponse;

use conduit::StatusCode;

#[derive(Debug)]
pub(super) struct Ok(pub(super) String);
#[derive(Debug)]
pub(super) struct BadRequest(pub(super) String);
#[derive(Debug)]
pub(super) struct ServerError(pub(super) String);

impl AppError for Ok {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.0, StatusCode::OK))
    }
}

impl fmt::Display for Ok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AppError for BadRequest {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.0, StatusCode::BAD_REQUEST))
    }
}

impl fmt::Display for BadRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AppError for ServerError {
    fn response(&self) -> Option<AppResponse> {
        Some(json_error(&self.0, StatusCode::INTERNAL_SERVER_ERROR))
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
