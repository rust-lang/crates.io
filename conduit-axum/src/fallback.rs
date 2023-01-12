use crate::error::ServiceError;

use std::error::Error;

use axum::extract::Extension;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use tracing::error;

#[derive(Clone, Debug)]
pub struct ErrorField(pub String);

#[derive(Clone, Debug)]
pub struct CauseField(pub String);

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        server_error_response(&self)
    }
}

/// Logs an error message and returns a generic status 500 response
pub fn server_error_response<E: Error + ?Sized>(error: &E) -> Response {
    error!(%error, "Internal Server Error");

    sentry_core::capture_error(error);

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Extension(ErrorField(error.to_string())),
        "Internal Server Error",
    )
        .into_response()
}
