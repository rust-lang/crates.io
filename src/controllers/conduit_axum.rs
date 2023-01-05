use crate::util::errors::BoxedAppError;
use axum::response::{IntoResponse, Response};
use conduit_axum::{spawn_blocking, ServiceError};

/// This runs the passed-in function in a synchronous [spawn_blocking] context
/// and converts any returned [AppError] into an axum [Response].
pub async fn conduit_compat<F, R>(f: F) -> Response
where
    F: FnOnce() -> Result<R, BoxedAppError> + Send + 'static,
    R: IntoResponse,
{
    spawn_blocking(move || f().into_response())
        .await
        .map_err(ServiceError::from)
        .into_response()
}
