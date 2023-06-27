use crate::env_optional;
use sentry::types::Dsn;
use sentry::IntoDsn;

pub struct SentryConfig {
    pub dsn: Option<Dsn>,
    pub environment: Option<String>,
    pub release: Option<String>,
    pub traces_sample_rate: f32,
}

impl SentryConfig {
    pub fn from_environment() -> Self {
        let dsn = dotenvy::var("SENTRY_DSN_API")
            .ok()
            .into_dsn()
            .expect("SENTRY_DSN_API is not a valid Sentry DSN value");

        let environment = dsn.as_ref().map(|_| {
            dotenvy::var("SENTRY_ENV_API")
                .expect("SENTRY_ENV_API must be set when using SENTRY_DSN_API")
        });

        Self {
            dsn,
            environment,
            release: dotenvy::var("HEROKU_SLUG_COMMIT").ok(),
            traces_sample_rate: env_optional("SENTRY_TRACES_SAMPLE_RATE").unwrap_or(0.0),
        }
    }
}
