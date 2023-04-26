use crate::env_optional;
use sentry::{ClientInitGuard, ClientOptions, IntoDsn, TransactionContext};
use std::sync::Arc;

/// Initializes the Sentry SDK from the environment variables.
///
/// If `SENTRY_DSN_API` is not set then Sentry will not be initialized,
/// otherwise it is required to be a valid DSN string. `SENTRY_ENV_API` must
/// be set if a DSN is provided.
///
/// `HEROKU_SLUG_COMMIT`, if present, will be used as the `release` property
/// on all events.
pub fn init() -> ClientInitGuard {
    let dsn = dotenv::var("SENTRY_DSN_API")
        .ok()
        .into_dsn()
        .expect("SENTRY_DSN_API is not a valid Sentry DSN value");

    let environment = dsn.as_ref().map(|_| {
        dotenv::var("SENTRY_ENV_API")
            .expect("SENTRY_ENV_API must be set when using SENTRY_DSN_API")
            .into()
    });

    let release = dotenv::var("HEROKU_SLUG_COMMIT").ok().map(Into::into);

    let traces_sample_rate = env_optional("SENTRY_TRACES_SAMPLE_RATE").unwrap_or(0.0);

    let traces_sampler = move |ctx: &TransactionContext| -> f32 {
        if let Some(sampled) = ctx.sampled() {
            return if sampled { 1.0 } else { 0.0 };
        }

        let op = ctx.operation();
        if op == "http.server" {
            let is_download_endpoint =
                ctx.name().starts_with("GET /api/v1/crates/") && ctx.name().ends_with("/download");

            if is_download_endpoint {
                // Reduce the sample rate for the download endpoint, since we have significantly
                // more traffic on that endpoint compared to the rest
                return traces_sample_rate / 100.;
            } else if ctx.name() == "PUT /api/v1/crates/new" {
                // Record all traces for crate publishing
                return 1.;
            } else if ctx.name().starts_with("GET /api/private/metrics/") {
                // Ignore all traces for internal metrics collection
                return 0.;
            }
        } else if op == "swirl.perform" || op == "admin.command" {
            // Record all traces for background tasks and admin commands
            return 1.;
        } else if op == "swirl.run" || op == "server.run" {
            // Ignore top-level span from the background worker and http server
            return 0.;
        }

        traces_sample_rate
    };

    let opts = ClientOptions {
        auto_session_tracking: true,
        dsn,
        environment,
        release,
        session_mode: sentry::SessionMode::Request,
        traces_sampler: Some(Arc::new(traces_sampler)),
        ..Default::default()
    };

    sentry::init(opts)
}
