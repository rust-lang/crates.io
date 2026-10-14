use tokio::task::JoinHandle;

/// Runs the provided closure on a thread where blocking is acceptable.
///
/// This is using [`tokio::task::spawn_blocking`] internally, but automatically
/// runs the callback function in the context of the current [`tracing::Span`].
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::errors::BoxedAppError;

    /// Tests that [`spawn_blocking`] works with [`anyhow`].
    #[tokio::test]
    async fn test_spawn_blocking_anyhow() {
        spawn_blocking(|| Ok::<_, anyhow::Error>(()))
            .await
            .unwrap()
            .unwrap()
    }

    /// Tests that [`spawn_blocking`] works with [`BoxedAppError`].
    #[tokio::test]
    async fn test_spawn_blocking_apperror() {
        spawn_blocking(|| Ok::<_, BoxedAppError>(()))
            .await
            .unwrap()
            .unwrap()
    }
}
