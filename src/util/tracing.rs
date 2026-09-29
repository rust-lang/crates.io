use crates_io_env_vars::var;
use sentry::integrations::tracing::EventFilter;
use serde_json::Value;
use std::backtrace::Backtrace;
use std::panic::{self, PanicHookInfo};
use tracing::{Level, Metadata, error, warn};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{EnvFilter, Layer, prelude::*};

/// Initializes the `tracing` logging framework.
///
/// Regular CLI output is influenced by the optional
/// [`RUST_LOG`](tracing_subscriber::filter::EnvFilter) environment variable
/// and is showing all `INFO` level events by default.
///
/// This function also sets up the Sentry error reporting integration for the
/// `tracing` framework, which is hardcoded to include all `INFO` level events.
///
/// Returns an error if the Heroku release version cannot be read in JSON mode.
pub fn init() -> anyhow::Result<()> {
    init_with_default_level(LevelFilter::INFO)
}

fn log_panic(info: &PanicHookInfo<'_>) {
    let message = info.payload_as_str().unwrap_or("non-string panic payload");
    let backtrace = Backtrace::force_capture();

    error!(
        target: "panic",
        { error.message = message, error.stack = %backtrace },
        "Process panicked: {message}"
    );
}

fn init_with_default_level(level: LevelFilter) -> anyhow::Result<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    let log_format = var("RUST_LOG_FORMAT")
        .inspect_err(|error| {
            warn!("Failed to read RUST_LOG_FORMAT, falling back to default: {error}")
        })
        .unwrap_or_default();

    let log_layer = match log_format.as_deref() {
        Some("json") => json_layer()?.with_filter(env_filter).boxed(),
        _ => tracing_subscriber::fmt::layer()
            .compact()
            .without_time()
            .with_filter(env_filter)
            .boxed(),
    };

    let sentry_layer = sentry::integrations::tracing::layer()
        .event_filter(event_filter)
        .with_filter(LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(log_layer)
        .with(sentry_layer)
        .init();

    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        log_panic(info);
        previous_hook(info);
    }));

    Ok(())
}

/// Builds the JSON log layer with the Heroku release version on each event.
fn json_layer() -> anyhow::Result<json_subscriber::fmt::Layer> {
    let mut layer = json_subscriber::fmt::layer()
        .flatten_event(true)
        .with_flat_span_list(true);

    let inner = layer.inner_layer_mut();

    if let Some(v) = crates_io_heroku::dyno_id()? {
        inner.add_static_field("heroku.dyno.id", Value::String(v));
    }
    if let Some(v) = crates_io_heroku::release_version()? {
        inner.add_static_field("heroku.release.version", Value::String(v));
    }
    if let Some(v) = crates_io_heroku::commit()? {
        inner.add_static_field("heroku.release.commit", Value::String(v));
    }

    Ok(layer)
}

pub fn event_filter(metadata: &Metadata<'_>) -> EventFilter {
    match metadata.level() {
        &Level::ERROR if metadata.target() == "panic" => EventFilter::Ignore,
        &Level::ERROR if metadata.target() == "http" => EventFilter::Breadcrumb,
        &Level::ERROR => EventFilter::Event,
        &Level::WARN | &Level::INFO => EventFilter::Breadcrumb,
        &Level::DEBUG | &Level::TRACE => EventFilter::Ignore,
    }
}

/// Initializes the `tracing` logging framework for usage in tests.
pub fn init_for_test() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy()
        .add_directive("tokio_postgres=info".parse().unwrap());

    let _ = tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .with_test_writer()
        .try_init();
}
