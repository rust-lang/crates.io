use crate::util::errors::AppResult;
use sentry::Hub;
use std::convert::identity;
use tokio::task::JoinHandle;

/// Just like [tokio::task::spawn_blocking], but automatically runs the passed
/// in function in the context of the current Sentry hub.
fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let hub = Hub::current();
    tokio::task::spawn_blocking(move || Hub::run(hub, f))
}

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
