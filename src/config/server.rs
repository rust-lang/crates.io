use anyhow::{anyhow, Context};
use ipnetwork::IpNetwork;
use oauth2::{ClientId, ClientSecret};

use crate::rate_limiter::{LimitedAction, RateLimiterConfig};
use crate::Env;

use super::base::Base;
use super::database_pools::DatabasePools;
use crate::config::cdn_log_storage::CdnLogStorageConfig;
use crate::config::CdnLogQueueConfig;
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
    pub max_upload_size: u32,
    pub max_unpack_size: u64,
    pub max_dependencies: usize,
    pub max_features: usize,
    pub rate_limiter: HashMap<LimitedAction, RateLimiterConfig>,
    pub new_version_rate_limit: Option<u32>,
    pub blocked_traffic: Vec<(String, Vec<String>)>,
    pub blocked_ips: HashSet<IpAddr>,
    pub max_allowed_page_offset: u32,
    pub page_offset_ua_blocklist: Vec<String>,
    pub page_offset_cidr_blocklist: Vec<IpNetwork>,
    pub excluded_crate_names: Vec<String>,
    pub domain_name: String,
    pub allowed_origins: AllowedOrigins,
    pub downloads_persist_interval: Duration,
    pub ownership_invitations_expiration_days: u64,
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

    pub content_security_policy: Option<HeaderValue>,
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
    /// - `BLOCKED_TRAFFIC`: A list of headers and environment variables to use for blocking
    ///   traffic. See the `block_traffic` module for more documentation.
    /// - `DOWNLOADS_PERSIST_INTERVAL_MS`: how frequent to persist download counts (in ms).
    /// - `METRICS_AUTHORIZATION_TOKEN`: authorization token needed to query metrics. If missing,
    ///   querying metrics will be completely disabled.
    /// - `WEB_MAX_ALLOWED_PAGE_OFFSET`: Page offsets larger than this value are rejected. Defaults
    ///   to 200.
    /// - `WEB_PAGE_OFFSET_UA_BLOCKLIST`: A comma separated list of user-agent substrings that will
    ///   be blocked if `WEB_MAX_ALLOWED_PAGE_OFFSET` is exceeded. Including an empty string in the
    ///   list will block *all* user-agents exceeding the offset. If not set or empty, no blocking
    ///   will occur.
    /// - `WEB_PAGE_OFFSET_CIDR_BLOCKLIST`: A comma separated list of CIDR blocks that will be used
    ///   to block IP addresses, e.g. `192.168.1.0/24`. If not set or empty, no blocking will occur.
    /// - `INSTANCE_METRICS_LOG_EVERY_SECONDS`: How frequently should instance metrics be logged.
    ///   If the environment variable is not present instance metrics are not logged.
    /// - `FORCE_UNCONDITIONAL_REDIRECTS`: Whether to force unconditional redirects in the download
    ///   endpoint even with a healthy database pool.
    /// - `BLOCKED_ROUTES`: A comma separated list of HTTP route patterns that are manually blocked
    ///   by an operator (e.g. `/crates/{crate_id}/{version}/download`).
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
        let page_offset_ua_blocklist = list("WEB_PAGE_OFFSET_UA_BLOCKLIST")?;
        let page_offset_cidr_blocklist =
            list_parsed("WEB_PAGE_OFFSET_CIDR_BLOCKLIST", parse_cidr_block)?;

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
        let content_security_policy = format!(
            "default-src 'self'; \
            connect-src 'self' *.ingest.sentry.io https://docs.rs https://play.rust-lang.org {cdn_domain}; \
            script-src 'self' 'unsafe-eval' 'sha256-n1+BB7Ckjcal1Pr7QNBh/dKRTtBQsIytFodRiIosXdE=' 'sha256-dbf9FMl76C7BnK1CC3eWb3pvsQAUaTYSHAlBy9tNTG0='; \
            style-src 'self' 'unsafe-inline' https://code.cdn.mozilla.net; \
            font-src https://code.cdn.mozilla.net; \
            img-src *; \
            object-src 'none'",
            cdn_domain = storage.cdn_prefix.as_ref().map(|cdn_prefix| format!("https://{cdn_prefix}")).unwrap_or_default()
        );

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
            max_upload_size: 10 * 1024 * 1024, // 10 MB default file upload size limit
            max_unpack_size: 512 * 1024 * 1024, // 512 MB max when decompressed
            max_dependencies: DEFAULT_MAX_DEPENDENCIES,
            max_features: DEFAULT_MAX_FEATURES,
            rate_limiter,
            new_version_rate_limit: var_parsed("MAX_NEW_VERSIONS_DAILY")?,
            blocked_traffic: blocked_traffic(),
            blocked_ips,
            max_allowed_page_offset: var_parsed("WEB_MAX_ALLOWED_PAGE_OFFSET")?.unwrap_or(200),
            page_offset_ua_blocklist,
            page_offset_cidr_blocklist,
            excluded_crate_names,
            domain_name: dotenvy::var("DOMAIN_NAME").unwrap_or_else(|_| "crates.io".into()),
            allowed_origins,
            downloads_persist_interval: var_parsed("DOWNLOADS_PERSIST_INTERVAL_MS")?
                .map(Duration::from_millis)
                .unwrap_or(Duration::from_secs(60)),
            ownership_invitations_expiration_days: 30,
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
            content_security_policy: Some(content_security_policy.parse()?),
        })
    }
}

impl Server {
    pub fn env(&self) -> Env {
        self.base.env
    }
}

/// Parses a CIDR block string to a valid `IpNetwork` struct.
///
/// The purpose is to be able to block IP ranges that overload the API that uses pagination.
///
/// The minimum number of bits for a host prefix must be
///
/// * at least 16 for IPv4 based CIDRs.
/// * at least 64 for IPv6 based CIDRs
///
fn parse_cidr_block(block: &str) -> anyhow::Result<IpNetwork> {
    let cidr = block
        .parse()
        .context("WEB_PAGE_OFFSET_CIDR_BLOCKLIST must contain IPv4 or IPv6 CIDR blocks.")?;

    let host_prefix = match cidr {
        IpNetwork::V4(_) => 16,
        IpNetwork::V6(_) => 64,
    };

    if cidr.prefix() < host_prefix {
        return Err(anyhow!("WEB_PAGE_OFFSET_CIDR_BLOCKLIST only allows CIDR blocks with a host prefix of at least 16 bits (IPv4) or 64 bits (IPv6)."));
    }

    Ok(cidr)
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

    #[test]
    fn parse_cidr_block_list_successfully() {
        assert_ok_eq!(
            parse_cidr_block("127.0.0.1/24"),
            "127.0.0.1/24".parse::<IpNetwork>().unwrap()
        );
        assert_ok_eq!(
            parse_cidr_block("192.168.0.1/31"),
            "192.168.0.1/31".parse::<IpNetwork>().unwrap()
        );
    }

    #[test]
    fn parse_cidr_blocks_panics_when_host_ipv4_prefix_is_too_low() {
        assert_err!(parse_cidr_block("127.0.0.1/8"));
    }

    #[test]
    fn parse_cidr_blocks_panics_when_host_ipv6_prefix_is_too_low() {
        assert_err!(parse_cidr_block(
            "2001:0db8:0123:4567:89ab:cdef:1234:5678/56"
        ));
    }

    #[test]
    fn parse_ipv6_based_cidr_blocks() {
        assert_ok_eq!(
            parse_cidr_block("2002::1234:abcd:ffff:c0a8:101/64"),
            "2002::1234:abcd:ffff:c0a8:101/64"
                .parse::<IpNetwork>()
                .unwrap()
        );
        assert_ok_eq!(
            parse_cidr_block("2001:0db8:0123:4567:89ab:cdef:1234:5678/92"),
            "2001:0db8:0123:4567:89ab:cdef:1234:5678/92"
                .parse::<IpNetwork>()
                .unwrap()
        );
    }
}
