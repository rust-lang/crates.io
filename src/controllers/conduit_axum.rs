use crate::util::errors::AppResult;
use conduit_axum::spawn_blocking;
use std::convert::identity;

/// This runs the passed-in function in a synchronous [spawn_blocking] context
/// and returns a flattened [AppResult].
pub async fn conduit_compat<F, R>(f: F) -> AppResult<R>
where
    F: FnOnce() -> AppResult<R> + Send + 'static,
    R: Send + 'static,
{
    spawn_blocking(f)
        .await
        // Convert `JoinError` to `BoxedAppError`
        .map_err(Into::into)
        // Flatten `Result<Result<_, E>, E>` to `Result<_, E>`
        .and_then(identity)
}
