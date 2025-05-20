#![doc = include_str!("../README.md")]

use anyhow::{Context, anyhow};
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

/// Reads an environment variable and parses it as a comma-separated list, or
/// returns an empty list if the variable is not set.
#[track_caller]
pub fn list(key: &str) -> anyhow::Result<Vec<String>> {
    let values = match var(key)? {
        None => vec![],
        Some(s) if s.is_empty() => vec![],
        Some(s) => s.split(',').map(str::trim).map(String::from).collect(),
    };

    Ok(values)
}

/// Reads an environment variable and parses it as a comma-separated list, or
/// returns an empty list if the variable is not set. Each individual value is
/// parsed using [FromStr].
#[track_caller]
pub fn list_parsed<T, E, F, C>(key: &str, f: F) -> anyhow::Result<Vec<T>>
where
    F: Fn(&str) -> C,
    C: Context<T, E>,
{
    let values = match var(key)? {
        None => vec![],
        Some(s) if s.is_empty() => vec![],
        Some(s) => s
            .split(',')
            .map(str::trim)
            .map(|s| {
                f(s).with_context(|| {
                    format!("Failed to parse value \"{s}\" of {key} environment variable")
                })
            })
            .collect::<Result<_, _>>()?,
    };

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::*;
    use std::sync::{LazyLock, Mutex};

    const TEST_VAR: &str = "CRATES_IO_ENV_VARS_TEST_VAR";

    /// A mutex to ensure that the tests don't run in parallel, since they all
    /// modify the shared environment variable.
    static MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn test_var() {
        let _guard = MUTEX.lock().unwrap();

        unsafe { std::env::set_var(TEST_VAR, "test") };
        assert_some_eq!(assert_ok!(var(TEST_VAR)), "test");

        unsafe { std::env::remove_var(TEST_VAR) };
        assert_none!(assert_ok!(var(TEST_VAR)));
    }

    #[test]
    fn test_required_var() {
        let _guard = MUTEX.lock().unwrap();

        unsafe { std::env::set_var(TEST_VAR, "test") };
        assert_ok_eq!(required_var(TEST_VAR), "test");

        unsafe { std::env::remove_var(TEST_VAR) };
        let error = assert_err!(required_var(TEST_VAR));
        assert_eq!(
            error.to_string(),
            "Failed to find required CRATES_IO_ENV_VARS_TEST_VAR environment variable"
        );
    }

    #[test]
    fn test_var_parsed() {
        let _guard = MUTEX.lock().unwrap();

        unsafe { std::env::set_var(TEST_VAR, "42") };
        assert_some_eq!(assert_ok!(var_parsed::<i32>(TEST_VAR)), 42);

        unsafe { std::env::set_var(TEST_VAR, "test") };
        let error = assert_err!(var_parsed::<i32>(TEST_VAR));
        assert_eq!(
            error.to_string(),
            "Failed to parse CRATES_IO_ENV_VARS_TEST_VAR environment variable"
        );

        unsafe { std::env::remove_var(TEST_VAR) };
        assert_none!(assert_ok!(var_parsed::<i32>(TEST_VAR)));
    }

    #[test]
    fn test_required_var_parsed() {
        let _guard = MUTEX.lock().unwrap();

        unsafe { std::env::set_var(TEST_VAR, "42") };
        assert_ok_eq!(required_var_parsed::<i32>(TEST_VAR), 42);

        unsafe { std::env::set_var(TEST_VAR, "test") };
        let error = assert_err!(required_var_parsed::<i32>(TEST_VAR));
        assert_eq!(
            error.to_string(),
            "Failed to parse CRATES_IO_ENV_VARS_TEST_VAR environment variable"
        );

        unsafe { std::env::remove_var(TEST_VAR) };
        let error = assert_err!(required_var_parsed::<i32>(TEST_VAR));
        assert_eq!(
            error.to_string(),
            "Failed to find required CRATES_IO_ENV_VARS_TEST_VAR environment variable"
        );
    }

    #[test]
    fn test_list() {
        let _guard = MUTEX.lock().unwrap();

        unsafe { std::env::set_var(TEST_VAR, "test") };
        assert_ok_eq!(list(TEST_VAR), vec!["test"]);

        unsafe { std::env::set_var(TEST_VAR, "test, foo,   bar   ") };
        assert_ok_eq!(list(TEST_VAR), vec!["test", "foo", "bar"]);

        unsafe { std::env::set_var(TEST_VAR, "") };
        assert_ok_eq!(list(TEST_VAR), Vec::<String>::new());

        unsafe { std::env::remove_var(TEST_VAR) };
        assert_ok_eq!(list(TEST_VAR), Vec::<String>::new());
    }

    #[test]
    fn test_list_parsed() {
        let _guard = MUTEX.lock().unwrap();

        unsafe { std::env::set_var(TEST_VAR, "42") };
        assert_ok_eq!(list_parsed(TEST_VAR, i32::from_str), vec![42]);

        unsafe { std::env::set_var(TEST_VAR, "42, 1,   -53   ") };
        assert_ok_eq!(list_parsed(TEST_VAR, i32::from_str), vec![42, 1, -53]);

        unsafe { std::env::set_var(TEST_VAR, "42, what") };
        let error = assert_err!(list_parsed(TEST_VAR, i32::from_str));
        assert_eq!(
            error.to_string(),
            "Failed to parse value \"what\" of CRATES_IO_ENV_VARS_TEST_VAR environment variable"
        );

        unsafe { std::env::set_var(TEST_VAR, "") };
        assert_ok_eq!(list_parsed(TEST_VAR, i32::from_str), Vec::<i32>::new());

        unsafe { std::env::remove_var(TEST_VAR) };
        assert_ok_eq!(list_parsed(TEST_VAR, i32::from_str), Vec::<i32>::new());
    }
}
