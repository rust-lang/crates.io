use oauth2::{ClientId, ClientSecret};
use secrecy::SecretString;
use url::Url;

use crate::Env;
use crate::util::gh_token_encryption::GitHubTokenEncryption;

use super::base::Base;
use super::database_pools::DatabasePools;
use crate::config::CdnLogQueueConfig;
use crate::config::block::BlockConfig;
use crate::config::cdn_log_storage::CdnLogStorageConfig;
use crate::config::datadog::DatadogConfig;
use crate::config::features::FeaturesConfig;
use crate::config::rate_limits::RateLimitsConfig;
use crate::middleware::cargo_compat::StatusCodeConfig;
use crate::storage::StorageConfig;
use crates_io_env_vars::{list, required_var, var, var_parsed};
use http::HeaderValue;
use std::convert::Infallible;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;

/// Maximum number of features a crate can have or that a feature itself can
/// enable. This value can be overridden in the database on a per-crate basis.
const DEFAULT_MAX_FEATURES: usize = 300;

/// Maximum number of dependencies a crate can have.
const DEFAULT_MAX_DEPENDENCIES: usize = 500;

pub struct Server {
    pub base: Base,
    pub ip: IpAddr,
    pub port: u16,
    pub max_blocking_threads: Option<usize>,
    pub db: DatabasePools,
    pub storage: StorageConfig,
    pub cdn_log_storage: CdnLogStorageConfig,
    pub cdn_log_queue: CdnLogQueueConfig,
    pub session_key: cookie::Key,
    pub gh_client_id: ClientId,
    pub gh_client_secret: ClientSecret,
    pub gh_token_encryption: GitHubTokenEncryption,
    pub max_upload_size: u32,
    pub max_unpack_size: u64,
    pub max_dependencies: usize,
    pub max_features: usize,
    pub rate_limits: RateLimitsConfig,
    pub block: BlockConfig,
    pub max_allowed_page_offset: u32,
    pub excluded_crate_names: Vec<String>,
    pub domain_name: String,
    pub allowed_origins: AllowedOrigins,
    pub ownership_invitations_expiration: chrono::Duration,
    pub metrics_authorization_token: Option<SecretString>,
    pub datadog: DatadogConfig,
    pub instance_metrics_log_every_seconds: Option<u64>,
    pub cdn_user_agent: String,

    /// Instructs the `cargo_compat` middleware whether to adjust response
    /// status codes to `200 OK` for all endpoints that are relevant for cargo.
    pub cargo_compat_status_code_config: StatusCodeConfig,

    /// Should the server serve the frontend assets in the `dist` directory?
    pub serve_dist: bool,

    /// Should the server serve the frontend `index.html` for all
    /// non-API requests?
    pub serve_html: bool,

    /// Base URL for the service from which the OpenGraph images
    /// for crates are loaded. Required if
    /// [`Self::serve_html`] is set.
    pub og_image_base_url: Option<Url>,

    /// Maximum number of items that the HTML render
    /// cache in `crate::middleware::frontend_html::serve`
    /// can hold. Defaults to 1024.
    pub html_render_cache_max_capacity: u64,

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
    /// Returns a default value for the application's config
    ///
    /// Sets the following default values:
    ///
    /// - `Config::max_upload_size`: 10MiB
    /// - `Config::ownership_invitations_expiration_days`: 30
    ///
    /// Pulls values from the following environment variables:
    ///
    /// - `SESSION_KEY`: The key used to sign and encrypt session cookies.
    /// - `GH_CLIENT_ID`: The client ID of the associated GitHub application.
    /// - `GH_CLIENT_SECRET`: The client secret of the associated GitHub application.
    /// - `GITHUB_TOKEN_ENCRYPTION_KEY`: Key for encrypting GitHub access tokens (64 hex characters).
    /// - `METRICS_AUTHORIZATION_TOKEN`: authorization token needed to query metrics. If missing,
    ///   querying metrics will be completely disabled.
    /// - `WEB_MAX_ALLOWED_PAGE_OFFSET`: Page offsets larger than this value are rejected. Defaults
    ///   to 200.
    /// - `INSTANCE_METRICS_LOG_EVERY_SECONDS`: How frequently should instance metrics be logged.
    ///   If the environment variable is not present instance metrics are not logged.
    /// - `FORCE_UNCONDITIONAL_REDIRECTS`: Whether to force unconditional redirects in the download
    ///   endpoint even with a healthy database pool.
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
        let docker = var("DEV_DOCKER")?.is_some();
        let heroku = var("HEROKU")?.is_some();

        let ip = if heroku || docker {
            [0, 0, 0, 0].into()
        } else {
            [127, 0, 0, 1].into()
        };

        let port = var_parsed("PORT")?.unwrap_or(8888);

        let allowed_origins = AllowedOrigins::from_default_env()?;

        let base = Base::from_environment()?;
        let excluded_crate_names = list("EXCLUDED_CRATE_NAMES")?;

        let max_blocking_threads = var_parsed("SERVER_THREADS")?;

        let storage = StorageConfig::from_environment();

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
            ip,
            port,
            max_blocking_threads,
            session_key: cookie::Key::derive_from(required_var("SESSION_KEY")?.as_bytes()),
            gh_client_id: ClientId::new(required_var("GH_CLIENT_ID")?),
            gh_client_secret: ClientSecret::new(required_var("GH_CLIENT_SECRET")?),
            gh_token_encryption: GitHubTokenEncryption::from_environment()?,
            max_upload_size: 10 * 1024 * 1024, // 10 MB default file upload size limit
            max_unpack_size: 512 * 1024 * 1024, // 512 MB max when decompressed
            max_dependencies: DEFAULT_MAX_DEPENDENCIES,
            max_features: DEFAULT_MAX_FEATURES,
            rate_limits: RateLimitsConfig::from_env()?,
            block: BlockConfig::from_env()?,
            max_allowed_page_offset: var_parsed("WEB_MAX_ALLOWED_PAGE_OFFSET")?.unwrap_or(200),
            excluded_crate_names,
            domain_name,
            allowed_origins,
            ownership_invitations_expiration: chrono::Duration::days(30),
            metrics_authorization_token: var("METRICS_AUTHORIZATION_TOKEN")?.map(Into::into),
            datadog: DatadogConfig::from_env()?,
            instance_metrics_log_every_seconds: var_parsed("INSTANCE_METRICS_LOG_EVERY_SECONDS")?,
            cdn_user_agent: var("WEB_CDN_USER_AGENT")?
                .unwrap_or_else(|| "Amazon CloudFront".into()),
            cargo_compat_status_code_config: var_parsed("CARGO_COMPAT_STATUS_CODES")?
                .unwrap_or(StatusCodeConfig::AdjustAll),
            serve_dist: true,
            serve_html: true,
            og_image_base_url: var_parsed("OG_IMAGE_BASE_URL")?,
            html_render_cache_max_capacity: var_parsed("HTML_RENDER_CACHE_CAP")?.unwrap_or(1024),
            trustpub_audience,
            disable_token_creation,
            banner_message,
            features: FeaturesConfig::from_env()?,
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
