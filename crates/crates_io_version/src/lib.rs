#![doc = include_str!("../README.md")]

/// Returns the Git SHA of the currently running deployment.
///
/// This function attempts to determine the commit SHA through various methods:
/// - Heroku environment variables (`HEROKU_BUILD_COMMIT`, `HEROKU_SLUG_COMMIT`)
///
/// Returns `None` if the commit SHA cannot be determined (e.g., in local
/// development environments).
///
/// # Examples
///
/// ```
/// use crates_io_version::commit;
///
/// if let Ok(Some(commit)) = commit() {
///     println!("Running commit: {}", commit);
/// } else {
///     println!("Commit SHA unknown");
/// }
/// ```
pub fn commit() -> anyhow::Result<Option<String>> {
    crates_io_heroku::commit()
}
