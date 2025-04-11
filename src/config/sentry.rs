use anyhow::Context;
use crates_io_env_vars::{required_var, var, var_parsed};
use sentry::IntoDsn;
use sentry::types::Dsn;

#[cfg_attr(test, derive(Default))]
pub struct SentryConfig {
    pub dsn: Option<Dsn>,
    pub environment: Option<String>,
    pub release: Option<String>,
    pub traces_sample_rate: f32,
}

impl SentryConfig {
    pub fn from_environment() -> anyhow::Result<Self> {
        let dsn = var("SENTRY_DSN_API")?
            .into_dsn()
            .context("SENTRY_DSN_API is not a valid Sentry DSN value")?;

        let environment = match dsn {
            None => None,
            Some(_) => Some(
                required_var("SENTRY_ENV_API")
                    .context("SENTRY_ENV_API must be set when using SENTRY_DSN_API")?,
            ),
        };

        Ok(Self {
            dsn,
            environment,
            release: var("HEROKU_SLUG_COMMIT")?,
            traces_sample_rate: var_parsed("SENTRY_TRACES_SAMPLE_RATE")?.unwrap_or(0.0),
        })
    }
}
