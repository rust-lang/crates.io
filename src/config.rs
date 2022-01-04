use crate::publish_rate_limit::PublishRateLimit;
use crate::{env, env_optional, uploaders::Uploader, Env};

mod base;
mod database_pools;

pub use self::base::Base;
pub use self::database_pools::DatabasePools;
use std::collections::HashSet;
use std::time::Duration;

const DEFAULT_VERSION_ID_CACHE_SIZE: u64 = 10_000;
const DEFAULT_VERSION_ID_CACHE_TTL: u64 = 5 * 60; // 5 minutes

pub struct Server {
    pub base: Base,
    pub db: DatabasePools,
    pub session_key: String,
    pub gh_client_id: String,
    pub gh_client_secret: String,
    pub gh_base_url: String,
    pub max_upload_size: u64,
    pub max_unpack_size: u64,
    pub publish_rate_limit: PublishRateLimit,
    pub blocked_traffic: Vec<(String, Vec<String>)>,
    pub max_allowed_page_offset: u32,
    pub page_offset_ua_blocklist: Vec<String>,
    pub domain_name: String,
    pub allowed_origins: Vec<String>,
    pub downloads_persist_interval_ms: usize,
    pub ownership_invitations_expiration_days: u64,
    pub metrics_authorization_token: Option<String>,
    pub use_test_database_pool: bool,
    pub instance_metrics_log_every_seconds: Option<u64>,
    pub force_unconditional_redirects: bool,
    pub blocked_routes: HashSet<String>,
    pub version_id_cache_size: u64,
    pub version_id_cache_ttl: Duration,
}

impl Default for Server {
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
    /// - `WEB_PAGE_OFFSET_UA_BLOCKLIST`: A comma seperated list of user-agent substrings that will
    ///   be blocked if `WEB_MAX_ALLOWED_PAGE_OFFSET` is exceeded. Including an empty string in the
    ///   list will block *all* user-agents exceeding the offset. If not set or empty, no blocking
    ///   will occur.
    /// - `INSTANCE_METRICS_LOG_EVERY_SECONDS`: How frequently should instance metrics be logged.
    ///   If the environment variable is not present instance metrics are not logged.
    /// - `FORCE_UNCONDITIONAL_REDIRECTS`: Whether to force unconditional redirects in the download
    ///   endpoint even with a healthy database pool.
    /// - `BLOCKED_ROUTES`: A comma separated list of HTTP route patterns that are manually blocked
    ///   by an operator (e.g. `/crates/:crate_id/:version/download`).
    ///
    /// # Panics
    ///
    /// This function panics if the Server configuration is invalid.
    fn default() -> Self {
        let allowed_origins = env("WEB_ALLOWED_ORIGINS")
            .split(',')
            .map(ToString::to_string)
            .collect();
        let page_offset_ua_blocklist = match env_optional::<String>("WEB_PAGE_OFFSET_UA_BLOCKLIST")
        {
            None => vec![],
            Some(s) if s.is_empty() => vec![],
            Some(s) => s.split(',').map(String::from).collect(),
        };
        Server {
            db: DatabasePools::full_from_environment(),
            base: Base::from_environment(),
            session_key: env("SESSION_KEY"),
            gh_client_id: env("GH_CLIENT_ID"),
            gh_client_secret: env("GH_CLIENT_SECRET"),
            gh_base_url: "https://api.github.com".to_string(),
            max_upload_size: 10 * 1024 * 1024, // 10 MB default file upload size limit
            max_unpack_size: 512 * 1024 * 1024, // 512 MB max when decompressed
            publish_rate_limit: Default::default(),
            blocked_traffic: blocked_traffic(),
            max_allowed_page_offset: env_optional("WEB_MAX_ALLOWED_PAGE_OFFSET").unwrap_or(200),
            page_offset_ua_blocklist,
            domain_name: domain_name(),
            allowed_origins,
            downloads_persist_interval_ms: dotenv::var("DOWNLOADS_PERSIST_INTERVAL_MS")
                .map(|interval| {
                    interval
                        .parse()
                        .expect("invalid DOWNLOADS_PERSIST_INTERVAL_MS")
                })
                .unwrap_or(60_000), // 1 minute
            ownership_invitations_expiration_days: 30,
            metrics_authorization_token: dotenv::var("METRICS_AUTHORIZATION_TOKEN").ok(),
            use_test_database_pool: false,
            instance_metrics_log_every_seconds: env_optional("INSTANCE_METRICS_LOG_EVERY_SECONDS"),
            force_unconditional_redirects: dotenv::var("FORCE_UNCONDITIONAL_REDIRECTS").is_ok(),
            blocked_routes: env_optional("BLOCKED_ROUTES")
                .map(|routes: String| routes.split(',').map(|s| s.into()).collect())
                .unwrap_or_else(HashSet::new),
            version_id_cache_size: env_optional("VERSION_ID_CACHE_SIZE")
                .unwrap_or(DEFAULT_VERSION_ID_CACHE_SIZE),
            version_id_cache_ttl: Duration::from_secs(
                env_optional("VERSION_ID_CACHE_TTL").unwrap_or(DEFAULT_VERSION_ID_CACHE_TTL),
            ),
        }
    }
}

impl Server {
    pub fn env(&self) -> Env {
        self.base.env
    }

    pub fn uploader(&self) -> &Uploader {
        self.base.uploader()
    }
}

pub(crate) fn domain_name() -> String {
    dotenv::var("DOMAIN_NAME").unwrap_or_else(|_| "crates.io".into())
}

fn blocked_traffic() -> Vec<(String, Vec<String>)> {
    let pattern_list = dotenv::var("BLOCKED_TRAFFIC").unwrap_or_default();
    parse_traffic_patterns(&pattern_list)
        .map(|(header, value_env_var)| {
            let value_list = dotenv::var(value_env_var).unwrap_or_default();
            let values = value_list.split(',').map(String::from).collect();
            (header.into(), values)
        })
        .collect()
}

fn parse_traffic_patterns(patterns: &str) -> impl Iterator<Item = (&str, &str)> {
    patterns.split_terminator(',').map(|pattern| {
        if let Some(idx) = pattern.find('=') {
            (&pattern[..idx], &pattern[(idx + 1)..])
        } else {
            panic!(
                "BLOCKED_TRAFFIC must be in the form HEADER=VALUE_ENV_VAR, \
                 got invalid pattern {}",
                pattern
            )
        }
    })
}

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
