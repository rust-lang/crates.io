#![doc = include_str!("../README.md")]

use crates_io_env_vars::var;

/// Returns the Git SHA of the currently deployed commit.
///
/// This value comes from the `HEROKU_SLUG_COMMIT` environment variable,
/// which is set by Heroku when the `runtime-dyno-metadata` Labs feature
/// is enabled. If the variable is not set (e.g., in local development
/// or when the feature is disabled), returns `None`.
///
/// Note: `HEROKU_SLUG_COMMIT` is deprecated by Heroku in favor of
/// `HEROKU_BUILD_COMMIT`, but this function continues to use
/// `HEROKU_SLUG_COMMIT` for backward compatibility with existing
/// deployments.
///
/// See <https://devcenter.heroku.com/articles/dyno-metadata> for more
/// information.
///
/// # Examples
///
/// ```
/// use crates_io_heroku::slug_commit;
///
/// if let Ok(Some(commit)) = slug_commit() {
///     println!("Running commit: {}", commit);
/// } else {
///     println!("Commit SHA unknown");
/// }
/// ```
pub fn slug_commit() -> anyhow::Result<Option<String>> {
    var("HEROKU_SLUG_COMMIT")
}

/// Returns the Git SHA of the currently deployed commit.
///
/// This value comes from the `HEROKU_BUILD_COMMIT` environment variable,
/// which is set by Heroku when the `runtime-dyno-build-metadata` Labs
/// feature is enabled. If the variable is not set (e.g., in local development
/// or when the feature is disabled), returns `None`.
///
/// This is the recommended function to use, as `HEROKU_BUILD_COMMIT` is
/// the current standard while `HEROKU_SLUG_COMMIT` is deprecated.
///
/// See <https://devcenter.heroku.com/articles/dyno-metadata> for more
/// information.
///
/// # Examples
///
/// ```
/// use crates_io_heroku::build_commit;
///
/// if let Ok(Some(commit)) = build_commit() {
///     println!("Running commit: {}", commit);
/// } else {
///     println!("Commit SHA unknown");
/// }
/// ```
pub fn build_commit() -> anyhow::Result<Option<String>> {
    var("HEROKU_BUILD_COMMIT")
}
