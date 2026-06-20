#[derive(Debug)]
pub struct PublishLimitsConfig {
    /// Maximum size in bytes of an uploaded crate file.
    pub upload_size: u32,

    /// Maximum size in bytes of a crate file once decompressed.
    pub unpack_size: u64,

    /// Maximum number of dependencies a crate can have.
    pub dependencies: usize,

    /// Maximum number of features a crate can have or that a feature itself can
    /// enable. This value can be overridden in the database on a per-crate basis.
    pub features: usize,
}

impl Default for PublishLimitsConfig {
    fn default() -> Self {
        Self {
            upload_size: 10 * 1024 * 1024,  // 10 MB
            unpack_size: 512 * 1024 * 1024, // 512 MB
            dependencies: 500,
            features: 300,
        }
    }
}

impl PublishLimitsConfig {
    /// Returns smaller limits suitable for use in tests.
    #[cfg(any(test, debug_assertions))]
    pub fn for_testing() -> Self {
        Self {
            upload_size: 128 * 1024, // 128 kB should be enough for most testing purposes
            unpack_size: 128 * 1024,
            dependencies: 10,
            features: 10,
        }
    }
}
