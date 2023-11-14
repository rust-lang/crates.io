use sentry::integrations::tracing::EventFilter;
use tracing::Level;
use tracing::Metadata;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{prelude::*, EnvFilter};

/// Initializes the `tracing` logging framework.
///
/// Regular CLI output is influenced by the optional
/// [`RUST_LOG`](tracing_subscriber::filter::EnvFilter) environment variable
/// and is showing all `INFO` level events by default.
///
/// This function also sets up the Sentry error reporting integration for the
/// `tracing` framework, which is hardcoded to include all `INFO` level events.
pub fn init() {
    init_with_default_level(LevelFilter::INFO)
}

fn init_with_default_level(level: LevelFilter) {
    let env_filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    let log_layer = tracing_subscriber::fmt::layer()
        .compact()
        .without_time()
        .with_filter(env_filter);

    let sentry_layer = sentry::integrations::tracing::layer()
        .event_filter(event_filter)
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(log_layer)
        .with(sentry_layer)
        .init();
}

pub fn event_filter(metadata: &Metadata<'_>) -> EventFilter {
    match metadata.level() {
        &Level::ERROR if metadata.target() == "http" => EventFilter::Breadcrumb,
        &Level::ERROR => EventFilter::Exception,
        &Level::WARN | &Level::INFO => EventFilter::Breadcrumb,
        &Level::DEBUG | &Level::TRACE => EventFilter::Ignore,
    }
}

/// Initializes the `tracing` logging framework for usage in tests.
pub fn init_for_test() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let _ = tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .without_time()
        .with_test_writer()
        .try_init();
}
