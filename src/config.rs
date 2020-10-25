use crate::publish_rate_limit::PublishRateLimit;
use crate::{env, uploaders::Uploader, Env, Replica};

#[derive(Clone, Debug)]
pub struct Config {
    pub uploader: Uploader,
    pub session_key: String,
    pub gh_client_id: String,
    pub gh_client_secret: String,
    pub db_url: String,
    pub replica_db_url: Option<String>,
    pub env: Env,
    pub max_upload_size: u64,
    pub max_unpack_size: u64,
    pub mirror: Replica,
    pub api_protocol: String,
    pub publish_rate_limit: PublishRateLimit,
    pub blocked_traffic: Vec<(String, Vec<String>)>,
    pub domain_name: String,
    pub allowed_origins: Vec<String>,
}

impl Default for Config {
    /// Returns a default value for the application's config
    ///
    /// Sets the following default values:
    ///
    /// - `Config::max_upload_size`: 10MiB
    /// - `Config::api_protocol`: `https`
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
    ///.  traffic. See the `block_traffic` module for more documentation.
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
        Config {
            uploader,
            session_key: env("SESSION_KEY"),
            gh_client_id: env("GH_CLIENT_ID"),
            gh_client_secret: env("GH_CLIENT_SECRET"),
            db_url: env("DATABASE_URL"),
            replica_db_url: dotenv::var("READ_ONLY_REPLICA_URL").ok(),
            env: cargo_env,
            max_upload_size: 10 * 1024 * 1024, // 10 MB default file upload size limit
            max_unpack_size: 512 * 1024 * 1024, // 512 MB max when decompressed
            mirror,
            api_protocol,
            publish_rate_limit: Default::default(),
            blocked_traffic: blocked_traffic(),
            domain_name: domain_name(),
            allowed_origins,
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
