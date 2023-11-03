use anyhow::{anyhow, Context};
use std::error::Error;
use std::str::FromStr;

/// Reads an environment variable for the current process.
///
/// Compared to [std::env::var] there are a couple of differences:
///
/// - [var] uses [dotenvy] which loads the `.env` file from the current or
///   parent directories before returning the value.
///
/// - [var] returns `Ok(None)` (instead of `Err`) if an environment variable
///   wasn't set.
#[track_caller]
pub fn var(key: &str) -> anyhow::Result<Option<String>> {
    match dotenvy::var(key) {
        Ok(content) => Ok(Some(content)),
        Err(dotenvy::Error::EnvVar(std::env::VarError::NotPresent)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Reads an environment variable for the current process, and fails if it was
/// not found.
///
/// Compared to [std::env::var] there are a couple of differences:
///
/// - [var] uses [dotenvy] which loads the `.env` file from the current or
///   parent directories before returning the value.
#[track_caller]
pub fn required_var(key: &str) -> anyhow::Result<String> {
    required(var(key), key)
}

/// Reads an environment variable for the current process, and parses it if
/// it is set.
///
/// Compared to [std::env::var] there are a couple of differences:
///
/// - [var] uses [dotenvy] which loads the `.env` file from the current or
///   parent directories before returning the value.
///
/// - [var] returns `Ok(None)` (instead of `Err`) if an environment variable
///   wasn't set.
#[track_caller]
pub fn var_parsed<R>(key: &str) -> anyhow::Result<Option<R>>
where
    R: FromStr,
    R::Err: Error + Send + Sync + 'static,
{
    match var(key) {
        Ok(Some(content)) => {
            Ok(Some(content.parse().with_context(|| {
                format!("Failed to parse {key} environment variable")
            })?))
        }
        Ok(None) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Reads an environment variable for the current process, and parses it if
/// it is set or fails otherwise.
///
/// Compared to [std::env::var] there are a couple of differences:
///
/// - [var] uses [dotenvy] which loads the `.env` file from the current or
///   parent directories before returning the value.
#[track_caller]
pub fn required_var_parsed<R>(key: &str) -> anyhow::Result<R>
where
    R: FromStr,
    R::Err: Error + Send + Sync + 'static,
{
    required(var_parsed(key), key)
}

fn required<T>(res: anyhow::Result<Option<T>>, key: &str) -> anyhow::Result<T> {
    match res {
        Ok(opt) => opt.ok_or_else(|| anyhow!("Failed to find required {key} environment variable")),
        Err(error) => Err(error),
    }
}
