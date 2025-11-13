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

/// Returns a User-Agent string identifying the crates.io service.
///
/// The User-Agent format is `crates.io/[commit] (https://crates.io)` when
/// the commit SHA is known, or `crates.io (https://crates.io)` when unknown.
///
/// The commit SHA is truncated to the first 7 characters.
pub fn user_agent() -> String {
    match commit() {
        Ok(Some(commit_sha)) => {
            let short_sha = commit_sha.chars().take(7).collect::<String>();
            format!("crates.io/{short_sha} (https://crates.io)")
        }
        _ => "crates.io (https://crates.io)".to_string(),
    }
}
