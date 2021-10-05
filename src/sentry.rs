use sentry::{ClientInitGuard, ClientOptions, IntoDsn};
use std::borrow::Cow;

/// Initializes the Sentry SDK from the environment variables.
///
/// If `SENTRY_DSN_API` is not set then Sentry will not be initialized,
/// otherwise it is required to be a valid DSN string. `SENTRY_ENV_API` must
/// be set if a DSN is provided.
///
/// `HEROKU_SLUG_COMMIT`, if present, will be used as the `release` property
/// on all events.
#[must_use]
pub fn init() -> Option<ClientInitGuard> {
    dotenv::var("SENTRY_DSN_API")
        .ok()
        .into_dsn()
        .expect("SENTRY_DSN_API is not a valid Sentry DSN value")
        .map(|dsn| {
            let environment = Some(
                dotenv::var("SENTRY_ENV_API")
                    .map(Cow::Owned)
                    .expect("SENTRY_ENV_API must be set when using SENTRY_DSN_API"),
            );

            let release = dotenv::var("HEROKU_SLUG_COMMIT").ok().map(Into::into);

            let opts = ClientOptions {
                dsn: Some(dsn),
                environment,
                release,
                ..Default::default()
            };

            sentry::init(opts)
        })
}
