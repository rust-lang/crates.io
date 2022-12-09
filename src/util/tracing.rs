use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{prelude::*, EnvFilter};

/// Initializes the `tracing` logging framework.
///
/// Regular CLI output is influenced by the
/// [`RUST_LOG`](tracing_subscriber::filter::EnvFilter) environment variable.
///
/// This function also sets up the Sentry error reporting integration for the
/// `tracing` framework, which is hardcoded to include all `INFO` level events.
pub fn init() {
    let log_layer = tracing_subscriber::fmt::layer()
        .compact()
        .without_time()
        .with_filter(EnvFilter::from_default_env());

    let sentry_layer = sentry::integrations::tracing::layer().with_filter(LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(log_layer)
        .with(sentry_layer)
        .init();
}

/// Initializes the `tracing` logging framework for usage in tests.
pub fn init_for_test() {
    let _ = tracing_subscriber::fmt()
        .compact()
        .with_max_level(tracing::Level::INFO)
        .without_time()
        .with_test_writer()
        .try_init();
}
