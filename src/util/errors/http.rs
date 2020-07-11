use std::fmt;

use super::{json_error, AppError};
use crate::util::AppResponse;

use conduit::StatusCode;

// The following structs are emtpy and do not provide a custom message to the user

#[derive(Debug)]
pub(super) struct Forbidden;
#[derive(Debug)]
pub struct NotFound;

impl AppError for Forbidden {
    fn response(&self) -> Option<AppResponse> {
        let detail = "must be logged in to perform that action";
        Some(json_error(detail, StatusCode::FORBIDDEN))
    }
}

impl fmt::Display for Forbidden {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "must be logged in to perform that action".fmt(f)
    }
}

// The following structs wrap a String and provide a custom message to the user

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
