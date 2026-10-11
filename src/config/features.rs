use crates_io_env_vars::var_bool;

#[derive(Debug, Default)]
pub struct FeaturesConfig {
    /// Require new GitHub users to complete the explicit signup flow.
    ///
    /// Read from the `EXPLICIT_SIGNUP_ENABLED` environment variable.
    pub explicit_signup_enabled: bool,

    /// Reuse the worker's git index clone for single-crate syncs without
    /// fetching first, and only fetch to retry after a failed push.
    ///
    /// Read from the `GIT_INDEX_LAZY_FETCH_ENABLED` environment variable.
    pub git_index_lazy_fetch_enabled: bool,
}

impl FeaturesConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let explicit_signup_enabled = var_bool("EXPLICIT_SIGNUP_ENABLED")?.unwrap_or(false);
        let git_index_lazy_fetch_enabled =
            var_bool("GIT_INDEX_LAZY_FETCH_ENABLED")?.unwrap_or(false);
        Ok(Self {
            explicit_signup_enabled,
            git_index_lazy_fetch_enabled,
        })
    }
}
