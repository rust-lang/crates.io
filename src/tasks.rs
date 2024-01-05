use sentry::Hub;
use std::convert::identity;
use tokio::task::JoinError;

/// Runs the provided closure on a thread where blocking is acceptable.
///
/// This is using [tokio::task::spawn_blocking] internally, but automatically
/// runs the callback function in the context of the current Sentry [Hub].
///
/// The function also returns a flattened [Result], which requires the error
/// variant of the [Result] to implement [From\<JoinError>].
pub async fn spawn_blocking<F, R, E>(f: F) -> Result<R, E>
where
    F: FnOnce() -> Result<R, E> + Send + 'static,
    R: Send + 'static,
    E: Send + From<JoinError> + 'static,
{
    let current_span = tracing::Span::current();
    let hub = Hub::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(|| Hub::run(hub, f)))
        .await
        // Convert `JoinError` to `E`
        .map_err(Into::into)
        // Flatten `Result<Result<_, E>, E>` to `Result<_, E>`
        .and_then(identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::errors::BoxedAppError;

    /// Test that [spawn_blocking] works with [anyhow].
    #[tokio::test]
    async fn test_spawn_blocking_anyhow() {
        spawn_blocking::<_, _, anyhow::Error>(|| Ok(()))
            .await
            .unwrap()
    }

    /// Test that [spawn_blocking] works with [BoxedAppError].
    #[tokio::test]
    async fn test_spawn_blocking_apperror() {
        spawn_blocking::<_, _, BoxedAppError>(|| Ok(()))
            .await
            .unwrap()
    }
}
