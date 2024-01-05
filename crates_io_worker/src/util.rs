use anyhow::anyhow;
use sentry_core::Hub;
use std::any::Any;
use std::future::Future;
use std::panic::PanicInfo;
use tokio::task::JoinError;

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
        .and_then(std::convert::identity)
}

pub async fn with_sentry_transaction<F, R, E, Fut>(
    transaction_name: &str,
    callback: F,
) -> Result<R, E>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<R, E>>,
{
    let hub = Hub::new_from_top(Hub::current());
    let _scope_guard = hub.push_scope();

    let tx_ctx = sentry_core::TransactionContext::new(transaction_name, "swirl.perform");
    let tx = sentry_core::start_transaction(tx_ctx);

    hub.configure_scope(|scope| scope.set_span(Some(tx.clone().into())));

    let result = callback().await;

    tx.set_status(match result.is_ok() {
        true => sentry_core::protocol::SpanStatus::Ok,
        false => sentry_core::protocol::SpanStatus::UnknownError,
    });
    tx.finish();

    result
}

/// Try to figure out what's in the box, and print it if we can.
///
/// The actual error type we will get from `panic::catch_unwind` is really poorly documented.
/// However, the `panic::set_hook` functions deal with a `PanicInfo` type, and its payload is
/// documented as "commonly but not always `&'static str` or `String`". So we can try all of those,
/// and give up if we didn't get one of those three types.
pub fn try_to_extract_panic_info(info: &(dyn Any + Send + 'static)) -> anyhow::Error {
    if let Some(x) = info.downcast_ref::<PanicInfo<'_>>() {
        anyhow!("job panicked: {x}")
    } else if let Some(x) = info.downcast_ref::<&'static str>() {
        anyhow!("job panicked: {x}")
    } else if let Some(x) = info.downcast_ref::<String>() {
        anyhow!("job panicked: {x}")
    } else {
        anyhow!("job panicked")
    }
}
