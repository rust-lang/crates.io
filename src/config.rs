use crate::publish_rate_limit::PublishRateLimit;
use crate::{env, env_optional, uploaders::Uploader, Env, Replica};

#[derive(Debug)]
pub struct Config {
    pub uploader: Uploader,
    pub session_key: String,
    pub gh_client_id: String,
    pub gh_client_secret: String,
    pub gh_base_url: String,
    pub db_primary_config: DbPoolConfig,
    pub db_replica_config: Option<DbPoolConfig>,
    pub env: Env,
    pub max_upload_size: u64,
    pub max_unpack_size: u64,
    pub mirror: Replica,
    pub api_protocol: String,
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
}

#[derive(Debug)]
pub struct DbPoolConfig {
    pub url: String,
    pub read_only_mode: bool,
}

impl Default for Config {
    /// Returns a default value for the application's config
    ///
    /// Sets the following default values:
    ///
    /// - `Config::max_upload_size`: 10MiB
    /// - `Config::api_protocol`: `https`
    /// - `Config::ownership_invitations_expiration_days`: 30
    ///
    /// Pulls values from the following environment variables:
    ///
    /// - `MIRROR`: Is this instance of cargo_registry a mirror of crates.io.
    /// - `HEROKU`: Is this instance of cargo_registry currently running on Heroku.
    /// - `S3_BUCKET`: The S3 bucket used to store crate files. If not present during development,
    ///    cargo_registry will fall back to a local uploader.
    /// - `S3_REGION`: The region in which the bucket was created. Optional if US standard.
    /// - `S3_ACCESS_KEY`: The access key to interact with S3. Optional if running a mirror.
    /// - `S3_SECRET_KEY`: The secret key to interact with S3. Optional if running a mirror.
    /// - `SESSION_KEY`: The key used to sign and encrypt session cookies.
    /// - `GH_CLIENT_ID`: The client ID of the associated GitHub application.
    /// - `GH_CLIENT_SECRET`: The client secret of the associated GitHub application.
    /// - `DATABASE_URL`: The URL of the postgres database to use.
    /// - `READ_ONLY_REPLICA_URL`: The URL of an optional postgres read-only replica database.
    /// - `BLOCKED_TRAFFIC`: A list of headers and environment variables to use for blocking
    ///   traffic. See the `block_traffic` module for more documentation.
    /// - `DOWNLOADS_PERSIST_INTERVAL_MS`: how frequent to persist download counts (in ms).
    /// - `METRICS_AUTHORIZATION_TOKEN`: authorization token needed to query metrics. If missing,
    ///   querying metrics will be completely disabled.
    /// - `DB_OFFLINE`: If set to `leader` then use the read-only follower as if it was the leader.
    ///   If set to `follower` then act as if `READ_ONLY_REPLICA_URL` was unset.
    /// - `READ_ONLY_MODE`: If defined (even as empty) then force all connections to be read-only.
    /// - `WEB_MAX_ALLOWED_PAGE_OFFSET`: Page offsets larger than this value are rejected. Defaults
    ///   to 200.
    /// - `WEB_PAGE_OFFSET_UA_BLOCKLIST`: A comma seperated list of user-agent substrings that will
    ///   be blocked if `WEB_MAX_ALLOWED_PAGE_OFFSET` is exceeded. Including an empty string in the
    ///   list will block *all* user-agents exceeding the offset. If not set or empty, no blocking
    ///   will occur.
    /// - `INSTANCE_METRICS_LOG_EVERY_SECONDS`: How frequently should instance metrics be logged.
    ///   If the environment variable is not present instance metrics are not logged.
    ///
    /// # Panics
    ///
    /// This function panics if `DB_OFFLINE=leader` but `READ_ONLY_REPLICA_URL` is unset.
    fn default() -> Config {
        let api_protocol = String::from("https");
        let mirror = if dotenv::var("MIRROR").is_ok() {
            Replica::ReadOnlyMirror
        } else {
            Replica::Primary
        };
        let heroku = dotenv::var("HEROKU").is_ok();
        let cargo_env = if heroku {
            Env::Production
        } else {
            Env::Development
        };

        let leader_url = env("DATABASE_URL");
        let follower_url = dotenv::var("READ_ONLY_REPLICA_URL").ok();
        let read_only_mode = dotenv::var("READ_ONLY_MODE").is_ok();
        let (db_primary_config, db_replica_config) = match dotenv::var("DB_OFFLINE").as_deref() {
            // The actual leader is down, use the follower in read-only mode as the primary and
            // don't configure a replica.
            Ok("leader") => (
                DbPoolConfig {
                    url: follower_url
                        .expect("Must set `READ_ONLY_REPLICA_URL` when using `DB_OFFLINE=leader`."),
                    read_only_mode: true,
                },
                None,
            ),
            // The follower is down, don't configure the replica.
            Ok("follower") => (
                DbPoolConfig {
                    url: leader_url,
                    read_only_mode,
                },
                None,
            ),
            _ => (
                DbPoolConfig {
                    url: leader_url,
                    read_only_mode,
                },
                follower_url.map(|url| DbPoolConfig {
                    url,
                    // Always enable read-only mode for the follower. In staging, we attach the
                    // same leader database to both environment variables and this ensures the
                    // connection is opened read-only even when attached to a writeable database.
                    read_only_mode: true,
                }),
            ),
        };

        let uploader = match (cargo_env, mirror) {
            (Env::Production, Replica::Primary) => {
                // `env` panics if these vars are not set, and in production for a primary instance,
                // that's what we want since we don't want to be able to start the server if the
                // server doesn't know where to upload crates.
                Uploader::S3 {
                    bucket: s3::Bucket::new(
                        env("S3_BUCKET"),
                        dotenv::var("S3_REGION").ok(),
                        env("S3_ACCESS_KEY"),
                        env("S3_SECRET_KEY"),
                        &api_protocol,
                    ),
                    cdn: dotenv::var("S3_CDN").ok(),
                }
            }
            (Env::Production, Replica::ReadOnlyMirror) => {
                // Read-only mirrors don't need access key or secret key since by definition,
                // they'll only need to read from a bucket, not upload.
                //
                // Read-only mirrors might have access key or secret key, so use them if those
                // environment variables are set.
                //
                // Read-only mirrors definitely need bucket though, so that they know where
                // to serve crate files from.
                Uploader::S3 {
                    bucket: s3::Bucket::new(
                        env("S3_BUCKET"),
                        dotenv::var("S3_REGION").ok(),
                        dotenv::var("S3_ACCESS_KEY").unwrap_or_default(),
                        dotenv::var("S3_SECRET_KEY").unwrap_or_default(),
                        &api_protocol,
                    ),
                    cdn: dotenv::var("S3_CDN").ok(),
                }
            }
            // In Development mode, either running as a primary instance or a read-only mirror
            _ => {
                if dotenv::var("S3_BUCKET").is_ok() {
                    // If we've set the `S3_BUCKET` variable to any value, use all of the values
                    // for the related S3 environment variables and configure the app to upload to
                    // and read from S3 like production does. All values except for bucket are
                    // optional, like production read-only mirrors.
                    println!("Using S3 uploader");
                    Uploader::S3 {
                        bucket: s3::Bucket::new(
                            env("S3_BUCKET"),
                            dotenv::var("S3_REGION").ok(),
                            dotenv::var("S3_ACCESS_KEY").unwrap_or_default(),
                            dotenv::var("S3_SECRET_KEY").unwrap_or_default(),
                            &api_protocol,
                        ),
                        cdn: dotenv::var("S3_CDN").ok(),
                    }
                } else {
                    // If we don't set the `S3_BUCKET` variable, we'll use a development-only
                    // uploader that makes it possible to run and publish to a locally-running
                    // crates.io instance without needing to set up an account and a bucket in S3.
                    println!(
                        "Using local uploader, crate files will be in the local_uploads directory"
                    );
                    Uploader::Local
                }
            }
        };
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
        Config {
            uploader,
            session_key: env("SESSION_KEY"),
            gh_client_id: env("GH_CLIENT_ID"),
            gh_client_secret: env("GH_CLIENT_SECRET"),
            gh_base_url: "https://api.github.com".to_string(),
            db_primary_config,
            db_replica_config,
            env: cargo_env,
            max_upload_size: 10 * 1024 * 1024, // 10 MB default file upload size limit
            max_unpack_size: 512 * 1024 * 1024, // 512 MB max when decompressed
            mirror,
            api_protocol,
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
        }
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
