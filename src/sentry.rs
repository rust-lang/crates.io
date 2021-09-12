use sentry::{ClientInitGuard, ClientOptions, IntoDsn};
use std::borrow::Cow;

#[must_use]
pub fn init() -> Option<ClientInitGuard> {
    dotenv::var("SENTRY_DSN_API")
        .ok()
        .into_dsn()
        .expect("SENTRY_DSN_API is not a valid Sentry DSN value")
        .map(|dsn| {
            let mut opts = ClientOptions::from(dsn);
            opts.environment = Some(
                dotenv::var("SENTRY_ENV_API")
                    .map(Cow::Owned)
                    .expect("SENTRY_ENV_API must be set when using SENTRY_DSN_API"),
            );

            opts.release = dotenv::var("HEROKU_SLUG_COMMIT").ok().map(Into::into);

            sentry::init(opts)
        })
}
