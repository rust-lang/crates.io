mod middleware;

pub use middleware::CustomSentryMiddleware as SentryMiddleware;
use sentry::{ClientInitGuard, ClientOptions, IntoDsn};

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

    let opts = ClientOptions {
        auto_session_tracking: true,
        dsn,
        environment,
        release,
        session_mode: sentry::SessionMode::Request,
        ..Default::default()
    };

    sentry::init(opts)
}
