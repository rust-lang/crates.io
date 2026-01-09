use oauth2::{ClientId, ClientSecret};
use url::Url;

use crate::Env;
use crate::rate_limiter::{LimitedAction, RateLimiterConfig};
use crate::util::gh_token_encryption::GitHubTokenEncryption;

use super::base::Base;
use super::database_pools::DatabasePools;
use crate::config::CdnLogQueueConfig;
use crate::config::cdn_log_storage::CdnLogStorageConfig;
use crate::middleware::cargo_compat::StatusCodeConfig;
use crate::storage::StorageConfig;
use crates_io_env_vars::{list, list_parsed, required_var, var, var_parsed};
use http::HeaderValue;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

const DEFAULT_VERSION_ID_CACHE_SIZE: u64 = 10_000;
const DEFAULT_VERSION_ID_CACHE_TTL: u64 = 5 * 60; // 5 minutes

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
    pub rate_limiter: HashMap<LimitedAction, RateLimiterConfig>,
    pub new_version_rate_limit: Option<u32>,
    pub blocked_traffic: Vec<(String, Vec<String>)>,
    pub blocked_ips: HashSet<IpAddr>,
    pub max_allowed_page_offset: u32,
    pub excluded_crate_names: Vec<String>,
    pub domain_name: String,
    pub allowed_origins: AllowedOrigins,
    pub downloads_persist_interval: Duration,
    pub ownership_invitations_expiration: chrono::Duration,
    pub metrics_authorization_token: Option<String>,
    pub instance_metrics_log_every_seconds: Option<u64>,
    pub blocked_routes: HashSet<String>,
    pub version_id_cache_size: u64,
    pub version_id_cache_ttl: Duration,
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
    /// cache in `crate::middleware::ember_html::serve_html`
    /// can hold. Defaults to 1024.
    pub html_render_cache_max_capacity: u64,

    pub content_security_policy: Option<HeaderValue>,

    /// The expected audience claim (`aud`) for the Trusted Publishing
    /// token exchange.
    pub trustpub_audience: String,

    /// Disables API token creation when set to any non-empty value.
    /// The value is used as the error message returned to users.
    pub disable_token_creation: Option<String>,

    /// Banner message to display on all pages (e.g., for security incidents).
    pub banner_message: Option<String>,

    /// Include publication timestamp in index entries (ISO8601 format).
    pub index_include_pubtime: bool,

    /// Enable Fastly CDN invalidation for sparse index files.
    pub sparse_index_fastly_enabled: bool,
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
    /// - `BLOCKED_TRAFFIC`: A list of headers and environment variables to use for blocking
    ///   traffic. See the `block_traffic` module for more documentation.
    /// - `DOWNLOADS_PERSIST_INTERVAL_MS`: how frequent to persist download counts (in ms).
    /// - `METRICS_AUTHORIZATION_TOKEN`: authorization token needed to query metrics. If missing,
    ///   querying metrics will be completely disabled.
    /// - `WEB_MAX_ALLOWED_PAGE_OFFSET`: Page offsets larger than this value are rejected. Defaults
    ///   to 200.
    /// - `INSTANCE_METRICS_LOG_EVERY_SECONDS`: How frequently should instance metrics be logged.
    ///   If the environment variable is not present instance metrics are not logged.
    /// - `FORCE_UNCONDITIONAL_REDIRECTS`: Whether to force unconditional redirects in the download
    ///   endpoint even with a healthy database pool.
    /// - `BLOCKED_ROUTES`: A comma separated list of HTTP route patterns that are manually blocked
    ///   by an operator (e.g. `/crates/{crate_id}/{version}/download`).
    /// - `DISABLE_TOKEN_CREATION`: If set to any non-empty value, disables API token creation
    ///   and uses the value as the error message returned to users.
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

        let blocked_ips = HashSet::from_iter(list_parsed("BLOCKED_IPS", IpAddr::from_str)?);

        let allowed_origins = AllowedOrigins::from_default_env()?;

        let base = Base::from_environment()?;
        let excluded_crate_names = list("EXCLUDED_CRATE_NAMES")?;

        let max_blocking_threads = var_parsed("SERVER_THREADS")?;

        // Dynamically load the configuration for all the rate limiting actions. See
        // `src/rate_limiter.rs` for their definition.
        let mut rate_limiter = HashMap::new();
        for action in LimitedAction::VARIANTS {
            let env_var_key = action.env_var_key();
            rate_limiter.insert(
                *action,
                RateLimiterConfig {
                    rate: Duration::from_secs(
                        var_parsed(&format!("RATE_LIMITER_{env_var_key}_RATE_SECONDS"))?
                            .unwrap_or_else(|| action.default_rate_seconds()),
                    ),
                    burst: var_parsed(&format!("RATE_LIMITER_{env_var_key}_BURST"))?
                        .unwrap_or_else(|| action.default_burst()),
                },
            );
        }

        let storage = StorageConfig::from_environment();

        // `sha256-dbf9FMl76C7BnK1CC3eWb3pvsQAUaTYSHAlBy9tNTG0=` refers to
        // the `script` in `public/github-redirect.html`
        // `sha256-qfh2Go0si80c5fCQM7vtMfMYVJPGGpVLgWpgpPssfvw=` refers to
        // the `script` in `public/github-auth-loading.html`
        let content_security_policy = format!(
            "default-src 'self'; \
            connect-src 'self' *.ingest.sentry.io https://docs.rs https://play.rust-lang.org https://raw.githubusercontent.com https://rustsec.org {cdn_domain}; \
            script-src 'self' 'unsafe-eval' 'sha256-n1+BB7Ckjcal1Pr7QNBh/dKRTtBQsIytFodRiIosXdE=' 'sha256-dbf9FMl76C7BnK1CC3eWb3pvsQAUaTYSHAlBy9tNTG0=' 'sha256-qfh2Go0si80c5fCQM7vtMfMYVJPGGpVLgWpgpPssfvw='; \
            style-src 'self' 'unsafe-inline' https://code.cdn.mozilla.net; \
            font-src https://code.cdn.mozilla.net; \
            img-src *; \
            object-src 'none'",
            cdn_domain = storage
                .cdn_prefix
                .as_ref()
                .map(|cdn_prefix| format!("https://{cdn_prefix}"))
                .unwrap_or_default()
        );

        let domain_name = dotenvy::var("DOMAIN_NAME").unwrap_or_else(|_| "crates.io".into());
        let trustpub_audience = var("TRUSTPUB_AUDIENCE")?.unwrap_or_else(|| domain_name.clone());
        let disable_token_creation = var("DISABLE_TOKEN_CREATION")?.filter(|s| !s.is_empty());
        let banner_message = var("BANNER_MESSAGE")?.filter(|s| !s.is_empty());
        let index_include_pubtime = var_parsed("INDEX_INCLUDE_PUBTIME")?.unwrap_or(false);

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
            rate_limiter,
            new_version_rate_limit: var_parsed("MAX_NEW_VERSIONS_DAILY")?,
            blocked_traffic: blocked_traffic(),
            blocked_ips,
            max_allowed_page_offset: var_parsed("WEB_MAX_ALLOWED_PAGE_OFFSET")?.unwrap_or(200),
            excluded_crate_names,
            domain_name,
            allowed_origins,
            downloads_persist_interval: var_parsed("DOWNLOADS_PERSIST_INTERVAL_MS")?
                .map(Duration::from_millis)
                .unwrap_or(Duration::from_secs(60)),
            ownership_invitations_expiration: chrono::Duration::days(30),
            metrics_authorization_token: var("METRICS_AUTHORIZATION_TOKEN")?,
            instance_metrics_log_every_seconds: var_parsed("INSTANCE_METRICS_LOG_EVERY_SECONDS")?,
            blocked_routes: HashSet::from_iter(list("BLOCKED_ROUTES")?),
            version_id_cache_size: var_parsed("VERSION_ID_CACHE_SIZE")?
                .unwrap_or(DEFAULT_VERSION_ID_CACHE_SIZE),
            version_id_cache_ttl: Duration::from_secs(
                var_parsed("VERSION_ID_CACHE_TTL")?.unwrap_or(DEFAULT_VERSION_ID_CACHE_TTL),
            ),
            cdn_user_agent: var("WEB_CDN_USER_AGENT")?
                .unwrap_or_else(|| "Amazon CloudFront".into()),
            cargo_compat_status_code_config: var_parsed("CARGO_COMPAT_STATUS_CODES")?
                .unwrap_or(StatusCodeConfig::AdjustAll),
            serve_dist: true,
            serve_html: true,
            og_image_base_url: var_parsed("OG_IMAGE_BASE_URL")?,
            html_render_cache_max_capacity: var_parsed("HTML_RENDER_CACHE_CAP")?.unwrap_or(1024),
            content_security_policy: Some(content_security_policy.parse()?),
            trustpub_audience,
            disable_token_creation,
            banner_message,
            index_include_pubtime,
            sparse_index_fastly_enabled: var_parsed("SPARSE_INDEX_FASTLY_ENABLED")?
                .unwrap_or(false),
        })
    }
}

impl Server {
    pub fn env(&self) -> Env {
        self.base.env
    }
}

fn blocked_traffic() -> Vec<(String, Vec<String>)> {
    let pattern_list = dotenvy::var("BLOCKED_TRAFFIC").unwrap_or_default();
    parse_traffic_patterns(&pattern_list)
        .map(|(header, value_env_var)| {
            let value_list = dotenvy::var(value_env_var).unwrap_or_default();
            let values = value_list.split(',').map(String::from).collect();
            (header.into(), values)
        })
        .collect()
}

fn parse_traffic_patterns(patterns: &str) -> impl Iterator<Item = (&str, &str)> {
    patterns.split_terminator(',').map(|pattern| {
        pattern.split_once('=').unwrap_or_else(|| {
            panic!(
                "BLOCKED_TRAFFIC must be in the form HEADER=VALUE_ENV_VAR, \
                 got invalid pattern {pattern}"
            )
        })
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_none;

    #[test]
    fn parse_traffic_patterns_splits_on_comma_and_looks_for_equal_sign() {
        let pattern_string_1 = "Foo=BAR,Bar=BAZ";
        let pattern_string_2 = "Baz=QUX";
        let pattern_string_3 = "";

        let patterns_1 = parse_traffic_patterns(pattern_string_1).collect::<Vec<_>>();
        assert_eq!(vec![("Foo", "BAR"), ("Bar", "BAZ")], patterns_1);

        let patterns_2 = parse_traffic_patterns(pattern_string_2).collect::<Vec<_>>();
        assert_eq!(vec![("Baz", "QUX")], patterns_2);

        assert_none!(parse_traffic_patterns(pattern_string_3).next());
    }
}
