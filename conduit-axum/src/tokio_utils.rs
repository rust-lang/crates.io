use sentry_core::Hub;
use tokio::task::JoinHandle;

/// Just like [tokio::task::spawn_blocking], but automatically runs the passed
/// in function in the context of the current Sentry hub.
pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let hub = Hub::current();
    tokio::task::spawn_blocking(move || Hub::run(hub, f))
}
