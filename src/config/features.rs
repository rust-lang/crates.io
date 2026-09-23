use crates_io_env_vars::var_parsed;

#[derive(Debug, Default)]
pub struct FeaturesConfig {
    /// Require new GitHub users to complete the explicit signup flow.
    ///
    /// Read from the `EXPLICIT_SIGNUP_ENABLED` environment variable.
    pub explicit_signup_enabled: bool,

    /// Fetch the git index only when the worker's cached clone is stale.
    ///
    /// Read from the `GIT_INDEX_LAZY_FETCH_ENABLED` environment variable.
    pub git_index_lazy_fetch_enabled: bool,
}

impl FeaturesConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let explicit_signup_enabled = var_parsed("EXPLICIT_SIGNUP_ENABLED")?.unwrap_or(false);
        let git_index_lazy_fetch_enabled =
            var_parsed("GIT_INDEX_LAZY_FETCH_ENABLED")?.unwrap_or(false);
        Ok(Self {
            explicit_signup_enabled,
            git_index_lazy_fetch_enabled,
        })
    }
}
