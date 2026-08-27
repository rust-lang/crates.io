use crates_io_env_vars::var_parsed;

#[derive(Debug, Default)]
pub struct FeaturesConfig {
    /// Require new GitHub users to complete the explicit signup flow.
    ///
    /// Read from the `EXPLICIT_SIGNUP_ENABLED` environment variable.
    pub explicit_signup_enabled: bool,
}

impl FeaturesConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let explicit_signup_enabled = var_parsed("EXPLICIT_SIGNUP_ENABLED")?.unwrap_or(false);
        Ok(Self {
            explicit_signup_enabled,
        })
    }
}
