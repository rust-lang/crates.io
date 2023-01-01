use crate::util::errors::AppError;
use axum::response::{IntoResponse, Response};
use conduit_axum::{conduit_into_axum, spawn_blocking, CauseField, ServiceError};

/// This runs the passed-in function in a synchronous [spawn_blocking] context
/// and converts any returned [AppError] into an axum [Response].
pub async fn conduit_compat<F, R>(f: F) -> Response
where
    F: FnOnce() -> Result<R, Box<dyn AppError>> + Send + 'static,
    R: IntoResponse,
{
    spawn_blocking(move || match f() {
        Ok(response) => response.into_response(),
        Err(error) => {
            let mut response = conduit_into_axum(error.response());

            if let Some(cause) = error.cause() {
                response
                    .extensions_mut()
                    .insert(CauseField(cause.to_string()));
            }

            response
        }
    })
    .await
    .map_err(ServiceError::from)
    .into_response()
}
