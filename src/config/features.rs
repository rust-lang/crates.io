use crates_io_env_vars::var_parsed;

#[derive(Debug, Default)]
pub struct FeaturesConfig {
    /// Require new GitHub users to complete the explicit signup flow.
    ///
    /// Read from the `EXPLICIT_SIGNUP_ENABLED` environment variable.
    pub explicit_signup_enabled: bool,

    /// Emit CDN cache tags as storage metadata on uploads.
    ///
    /// Read from the `CACHE_TAGS_ENABLED` environment variable.
    pub cache_tags_enabled: bool,

    /// Invalidate deleted crate objects using CDN cache tags instead of URLs.
    ///
    /// Read from the `CACHE_TAG_INVALIDATIONS_ENABLED` environment variable.
    pub cache_tag_invalidations_enabled: bool,
}

impl FeaturesConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let explicit_signup_enabled = var_parsed("EXPLICIT_SIGNUP_ENABLED")?.unwrap_or(false);
        let cache_tags_enabled = var_parsed("CACHE_TAGS_ENABLED")?.unwrap_or(false);
        let cache_tag_invalidations_enabled =
            var_parsed("CACHE_TAG_INVALIDATIONS_ENABLED")?.unwrap_or(false);

        if cache_tag_invalidations_enabled && !cache_tags_enabled {
            anyhow::bail!("CACHE_TAG_INVALIDATIONS_ENABLED requires CACHE_TAGS_ENABLED");
        }

        Ok(Self {
            explicit_signup_enabled,
            cache_tags_enabled,
            cache_tag_invalidations_enabled,
        })
    }
}
