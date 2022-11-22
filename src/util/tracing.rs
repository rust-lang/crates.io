use std::env;
use tracing::Level;
use tracing_subscriber::{filter, prelude::*};

/// Initializes the `tracing` logging framework.
///
/// Regular CLI output is influenced by the
/// [`RUST_LOG`](tracing_subscriber::filter::EnvFilter) environment variable.
///
/// This function also sets up the Sentry error reporting integration for the
/// `tracing` framework, which is hardcoded to include all `INFO` level events.
pub fn init() {
    let log_filter = env::var("RUST_LOG")
        .unwrap_or_default()
        .parse::<filter::Targets>()
        .expect("Invalid RUST_LOG value");

    let sentry_filter = filter::Targets::new().with_default(Level::INFO);

    tracing_subscriber::registry()
        .with(tracing_logfmt::layer().with_filter(log_filter))
        .with(sentry::integrations::tracing::layer().with_filter(sentry_filter))
        .init();
}
