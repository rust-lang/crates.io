#![doc = include_str!("../README.md")]

use crates_io_env_vars::var;

/// Returns whether the process is running on Heroku, as indicated by `HEROKU`.
pub fn is_heroku() -> anyhow::Result<bool> {
    Ok(var("HEROKU")?.is_some())
}

/// Returns the dyno name from `DYNO`, or `None` when unset.
///
/// An example value may be: `"web.1"`.
pub fn dyno() -> anyhow::Result<Option<String>> {
    var("DYNO")
}

/// Returns the dyno UUID from `HEROKU_DYNO_ID`, or `None` when unset.
///
/// An example value may be: `"1vac4117-c29f-4312-521e-ba4d8638c1ac"`.
pub fn dyno_id() -> anyhow::Result<Option<String>> {
    var("HEROKU_DYNO_ID")
}

/// Returns the release identifier from `HEROKU_RELEASE_VERSION`, or `None` when
/// unset.
///
/// An example value may be: `"v42"`.
pub fn release_version() -> anyhow::Result<Option<String>> {
    var("HEROKU_RELEASE_VERSION")
}

/// Returns the application UUID from `HEROKU_APP_ID`, or `None` when unset.
///
/// An example value may be: `"9daa2797-e49b-4624-932f-ec3f9688e3da"`.
pub fn app_id() -> anyhow::Result<Option<String>> {
    var("HEROKU_APP_ID")
}

/// Returns the release timestamp from `HEROKU_RELEASE_CREATED_AT`, or `None`
/// when unset.
///
/// An example value may be: `"2015-04-02T18:00:42Z"`.
pub fn release_created_at() -> anyhow::Result<Option<String>> {
    var("HEROKU_RELEASE_CREATED_AT")
}

/// Returns the Git SHA of the currently deployed commit.
///
/// This function tries `HEROKU_BUILD_COMMIT` first (the current standard),
/// and falls back to `HEROKU_SLUG_COMMIT` (deprecated) if the former is not
/// set. This provides compatibility with both old and new Heroku deployments.
///
/// Both environment variables are set by Heroku when the appropriate Labs
/// features are enabled (`runtime-dyno-build-metadata` for `HEROKU_BUILD_COMMIT`,
/// `runtime-dyno-metadata` for `HEROKU_SLUG_COMMIT`).
///
/// Returns `None` if neither variable is set (e.g., in local development).
///
/// See <https://devcenter.heroku.com/articles/dyno-metadata> for more
/// information.
///
/// # Examples
///
/// ```
/// use crates_io_heroku::commit;
///
/// if let Ok(Some(commit)) = commit() {
///     println!("Running commit: {}", commit);
/// } else {
///     println!("Commit SHA unknown");
/// }
/// ```
pub fn commit() -> anyhow::Result<Option<String>> {
    // Try the current standard first
    if let Some(commit) = build_commit()? {
        return Ok(Some(commit));
    }

    // Fall back to the deprecated variable for backward compatibility
    slug_commit()
}

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
