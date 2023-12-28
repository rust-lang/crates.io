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

#[cfg(test)]
mod tests {
    use super::*;
    use claims::*;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    const TEST_VAR: &str = "CRATES_IO_ENV_VARS_TEST_VAR";

    /// A mutex to ensure that the tests don't run in parallel, since they all
    /// modify the shared environment variable.
    static MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn test_var() {
        let _guard = MUTEX.lock().unwrap();

        std::env::set_var(TEST_VAR, "test");
        assert_some_eq!(assert_ok!(var(TEST_VAR)), "test");

        std::env::remove_var(TEST_VAR);
        assert_none!(assert_ok!(var(TEST_VAR)));
    }

    #[test]
    fn test_required_var() {
        let _guard = MUTEX.lock().unwrap();

        std::env::set_var(TEST_VAR, "test");
        assert_ok_eq!(required_var(TEST_VAR), "test");

        std::env::remove_var(TEST_VAR);
        let error = assert_err!(required_var(TEST_VAR));
        assert_eq!(
            error.to_string(),
            "Failed to find required CRATES_IO_ENV_VARS_TEST_VAR environment variable"
        );
    }

    #[test]
    fn test_var_parsed() {
        let _guard = MUTEX.lock().unwrap();

        std::env::set_var(TEST_VAR, "42");
        assert_some_eq!(assert_ok!(var_parsed::<i32>(TEST_VAR)), 42);

        std::env::set_var(TEST_VAR, "test");
        let error = assert_err!(var_parsed::<i32>(TEST_VAR));
        assert_eq!(
            error.to_string(),
            "Failed to parse CRATES_IO_ENV_VARS_TEST_VAR environment variable"
        );

        std::env::remove_var(TEST_VAR);
        assert_none!(assert_ok!(var_parsed::<i32>(TEST_VAR)));
    }

    #[test]
    fn test_required_var_parsed() {
        let _guard = MUTEX.lock().unwrap();

        std::env::set_var(TEST_VAR, "42");
        assert_ok_eq!(required_var_parsed::<i32>(TEST_VAR), 42);

        std::env::set_var(TEST_VAR, "test");
        let error = assert_err!(required_var_parsed::<i32>(TEST_VAR));
        assert_eq!(
            error.to_string(),
            "Failed to parse CRATES_IO_ENV_VARS_TEST_VAR environment variable"
        );

        std::env::remove_var(TEST_VAR);
        let error = assert_err!(required_var_parsed::<i32>(TEST_VAR));
        assert_eq!(
            error.to_string(),
            "Failed to find required CRATES_IO_ENV_VARS_TEST_VAR environment variable"
        );
    }
}
