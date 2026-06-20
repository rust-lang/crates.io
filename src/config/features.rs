use crates_io_env_vars::var_parsed;

#[derive(Debug, Default)]
pub struct FeaturesConfig {
    /// Include publication timestamps in index entries (ISO8601 format).
    ///
    /// Read from the `INDEX_INCLUDE_PUBTIME` environment variable.
    pub index_include_pubtime: bool,

    /// Enable enqueueing of `BuildCrateZip` jobs in the publish flow.
    ///
    /// Read from the `ZIP_ARCHIVES_ENABLED` environment variable.
    pub zip_archives_enabled: bool,
}

impl FeaturesConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let index_include_pubtime = var_parsed("INDEX_INCLUDE_PUBTIME")?.unwrap_or(false);
        let zip_archives_enabled = var_parsed("ZIP_ARCHIVES_ENABLED")?.unwrap_or(false);

        Ok(Self {
            index_include_pubtime,
            zip_archives_enabled,
        })
    }
}
