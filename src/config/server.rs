use url::Url;

use crate::Env;
use crate::util::gh_token_encryption::GitHubTokenEncryption;

use super::base::Base;
use super::database_pools::DatabasePools;
use crate::config::CdnLogQueueConfig;
use crate::config::bind::BindConfig;
use crate::config::block::BlockConfig;
use crate::config::cdn_log_storage::CdnLogStorageConfig;
use crate::config::datadog::DatadogConfig;
use crate::config::features::FeaturesConfig;
use crate::config::frontend::FrontendConfig;
use crate::config::github::GitHubOAuthConfig;
use crate::config::metrics::MetricsConfig;
use crate::config::publish_limits::PublishLimitsConfig;
use crate::config::rate_limits::RateLimitsConfig;
use crate::middleware::cargo_compat::StatusCodeConfig;
use crate::storage::StorageConfig;
use crates_io_env_vars::{list, required_var, var, var_parsed};
use http::HeaderValue;
use std::convert::Infallible;
use std::path::PathBuf;
use std::str::FromStr;

pub struct Server {
    pub base: Base,
    pub bind: BindConfig,
    pub max_blocking_threads: Option<usize>,
    pub db: DatabasePools,
    pub storage: StorageConfig,
    pub cdn_log_storage: CdnLogStorageConfig,
    pub cdn_log_queue: CdnLogQueueConfig,
    pub session_key: cookie::Key,
    pub github_oauth: GitHubOAuthConfig,
    pub gh_token_encryption: GitHubTokenEncryption,
    pub publish_limits: PublishLimitsConfig,
    pub rate_limits: RateLimitsConfig,
    pub block: BlockConfig,
    pub max_allowed_page_offset: u32,
    pub excluded_crate_names: Vec<String>,
    pub domain_name: String,
    pub allowed_origins: AllowedOrigins,
    pub ownership_invitations_expiration: chrono::Duration,
    pub metrics: MetricsConfig,
    pub datadog: DatadogConfig,
    pub cdn_user_agent: String,

    /// Instructs the `cargo_compat` middleware whether to adjust response
    /// status codes to `200 OK` for all endpoints that are relevant for cargo.
    pub cargo_compat_status_code_config: StatusCodeConfig,

    pub frontend: FrontendConfig,

    /// The expected audience claim (`aud`) for the Trusted Publishing
    /// token exchange.
    pub trustpub_audience: String,

    /// Disables API token creation when set to any non-empty value.
    /// The value is used as the error message returned to users.
    pub disable_token_creation: Option<String>,

    /// Banner message to display on all pages (e.g., for security incidents).
    pub banner_message: Option<String>,

    pub features: FeaturesConfig,

    /// URL of a git repository to mirror the crate index's snapshot branches
    /// to. When set, the `ArchiveIndexBranch` background job pushes snapshot
    /// branches to this remote; when unset the job is a no-op.
    pub index_archive_url: Option<Url>,

    /// Optional directory containing the `pg_dump` and `psql` binaries to use
    /// for database dump generation. When unset, the binaries are resolved via
    /// `PATH`.
    pub postgres_bin_dir: Option<PathBuf>,
}

impl Server {
    /// Returns a default value for the application's config.
    ///
    /// Sets the following default values:
    ///
    /// - `PublishLimitsConfig::upload_size`: 10MiB
    /// - `Server::ownership_invitations_expiration`: 30 days
    ///
    /// Pulls values from the following environment variables:
    ///
    /// - `SESSION_KEY`: The key used to sign and encrypt session cookies.
    /// - `GITHUB_TOKEN_ENCRYPTION_KEY`: Key for encrypting GitHub access tokens (64 hex characters).
    /// - `WEB_MAX_ALLOWED_PAGE_OFFSET`: Page offsets larger than this value are rejected. Defaults
    ///   to 200.
    /// - `DISABLE_TOKEN_CREATION`: If set to any non-empty value, disables API token creation
    ///   and uses the value as the error message returned to users.
    /// - `GIT_ARCHIVE_REPO_URL`: HTTPS URL (e.g. `https://github.com/<org>/<repo>.git`) of a git
    ///   repository to mirror the crate index's snapshot branches to. Must be HTTPS because the
    ///   `ArchiveIndexBranch` job authenticates via a GitHub App installation token; SSH remotes
    ///   are not supported. If unset the job is a no-op.
    /// - `POSTGRES_BIN_DIR`: Optional directory containing `pg_dump` and `psql` binaries to use
    ///   for database dump generation. If unset, the binaries are looked up via `PATH`.
    ///
    /// # Panics
    ///
    /// This function panics if the Server configuration is invalid.
    pub fn from_environment() -> anyhow::Result<Self> {
        let allowed_origins = AllowedOrigins::from_default_env()?;

        let base = Base::from_environment()?;
        let excluded_crate_names = list("EXCLUDED_CRATE_NAMES")?;

        let max_blocking_threads = var_parsed("SERVER_THREADS")?;

        let features = FeaturesConfig::from_env()?;

        let mut storage = StorageConfig::from_environment();
        storage.cache_tags_enabled = features.cache_tags_enabled;

        let domain_name = dotenvy::var("DOMAIN_NAME").unwrap_or_else(|_| "crates.io".into());
        let trustpub_audience = var("TRUSTPUB_AUDIENCE")?.unwrap_or_else(|| domain_name.clone());
        let disable_token_creation = var("DISABLE_TOKEN_CREATION")?.filter(|s| !s.is_empty());
        let banner_message = var("BANNER_MESSAGE")?.filter(|s| !s.is_empty());

        Ok(Server {
            db: DatabasePools::full_from_environment(&base)?,
            storage,
            cdn_log_storage: CdnLogStorageConfig::from_env()?,
            cdn_log_queue: CdnLogQueueConfig::from_env()?,
            base,
            bind: BindConfig::from_env()?,
            max_blocking_threads,
            session_key: cookie::Key::derive_from(required_var("SESSION_KEY")?.as_bytes()),
            github_oauth: GitHubOAuthConfig::from_env()?,
            gh_token_encryption: GitHubTokenEncryption::from_environment()?,
            publish_limits: PublishLimitsConfig::default(),
            rate_limits: RateLimitsConfig::from_env()?,
            block: BlockConfig::from_env()?,
            max_allowed_page_offset: var_parsed("WEB_MAX_ALLOWED_PAGE_OFFSET")?.unwrap_or(200),
            excluded_crate_names,
            domain_name,
            allowed_origins,
            ownership_invitations_expiration: chrono::Duration::days(30),
            metrics: MetricsConfig::from_env()?,
            datadog: DatadogConfig::from_env()?,
            cdn_user_agent: var("WEB_CDN_USER_AGENT")?
                .unwrap_or_else(|| "Amazon CloudFront".into()),
            cargo_compat_status_code_config: var_parsed("CARGO_COMPAT_STATUS_CODES")?
                .unwrap_or(StatusCodeConfig::AdjustAll),
            frontend: FrontendConfig::from_env()?,
            trustpub_audience,
            disable_token_creation,
            banner_message,
            features,
            index_archive_url: var_parsed("GIT_ARCHIVE_REPO_URL")?,
            postgres_bin_dir: var_parsed("POSTGRES_BIN_DIR")?,
        })
    }
}

impl Server {
    pub fn env(&self) -> Env {
        self.base.env
    }
}

#[derive(Clone, Debug, Default)]
pub struct AllowedOrigins(Vec<String>);

impl AllowedOrigins {
    pub fn from_str(s: &str) -> Self {
        Self(s.split(',').map(ToString::to_string).collect())
    }

    pub fn from_default_env() -> anyhow::Result<Self> {
        Ok(Self::from_str(&required_var("WEB_ALLOWED_ORIGINS")?))
    }

    pub fn contains(&self, value: &HeaderValue) -> bool {
        self.0.iter().any(|it| it == value)
    }
}

impl FromStr for AllowedOrigins {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_str(s))
    }
}
