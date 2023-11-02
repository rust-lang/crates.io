use crate::config::SentryConfig;
use sentry::{ClientInitGuard, ClientOptions, TransactionContext};
use std::sync::Arc;

/// Initializes the Sentry SDK from the environment variables.
///
/// If `SENTRY_DSN_API` is not set then Sentry will not be initialized,
/// otherwise it is required to be a valid DSN string. `SENTRY_ENV_API` must
/// be set if a DSN is provided.
///
/// `HEROKU_SLUG_COMMIT`, if present, will be used as the `release` property
/// on all events.
pub fn init() -> Option<ClientInitGuard> {
    let config = match SentryConfig::from_environment() {
        Ok(config) => config,
        Err(error) => {
            warn!(%error, "Failed to read Sentry configuration from environment");
            return None;
        }
    };

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
                return config.traces_sample_rate / 100.;
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

        config.traces_sample_rate
    };

    let opts = ClientOptions {
        auto_session_tracking: true,
        dsn: config.dsn,
        environment: config.environment.map(Into::into),
        release: config.release.map(Into::into),
        session_mode: sentry::SessionMode::Request,
        traces_sampler: Some(Arc::new(traces_sampler)),
        ..Default::default()
    };

    Some(sentry::init(opts))
}
