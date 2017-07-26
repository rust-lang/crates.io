use s3;

use std::env;
use std::path::PathBuf;

use {Env, env, Uploader, Replica};

#[derive(Clone, Debug)]
pub struct Config {
    pub uploader: Uploader,
    pub session_key: String,
    pub git_repo_checkout: PathBuf,
    pub gh_client_id: String,
    pub gh_client_secret: String,
    pub db_url: String,
    pub env: ::Env,
    pub max_upload_size: u64,
    pub mirror: Replica,
    pub api_protocol: String,
}

impl Default for Config {
    /// Returns a default value for the application's config
    ///
    /// Sets the following default values:
    /// - `Config::max_upload_size`: 10MiB
    /// - `Config::api_protocol`: `https`
    ///
    /// Pulls values from the following environment variables:
    /// - `GIT_REPO_CHECKOUT`: The directory where the registry index was cloned.
    /// - `MIRROR`: Is this instance of cargo_registry a mirror of crates.io.
    /// - `HEROKU`: Is this instance of cargo_registry currently running on Heroku.
    /// - `S3_BUCKET`: The S3 bucket used to store crate files. If not present during development,
    /// cargo_registry will fall back to a local uploader.
    /// - `S3_REGION`: The region in which the bucket was created. Optional if US standard.
    /// - `S3_ACCESS_KEY`: The access key to interact with S3. Optional if running a mirror.
    /// - `S3_SECRET_KEY`: The secret key to interact with S3. Optional if running a mirror.
    /// - `SESSION_KEY`: The key used to sign and encrypt session cookies.
    /// - `GH_CLIENT_ID`: The client ID of the associated GitHub application.
    /// - `GH_CLIENT_SECRET`: The client secret of the associated GitHub application.
    /// - `DATABASE_URL`: The URL of the postgres database to use.
    fn default() -> Config {
        let checkout = PathBuf::from(env("GIT_REPO_CHECKOUT"));
        let api_protocol = String::from("https");
        let mirror = if env::var("MIRROR").is_ok() {
            Replica::ReadOnlyMirror
        } else {
            Replica::Primary
        };
        let heroku = env::var("HEROKU").is_ok();
        let cargo_env = if heroku {
            Env::Production
        } else {
            Env::Development
        };
        let uploader = match (cargo_env, mirror) {
            (Env::Production, Replica::Primary) => {
                // `env` panics if these vars are not set
                Uploader::S3 {
                    bucket: s3::Bucket::new(
                        env("S3_BUCKET"),
                        env::var("S3_REGION").ok(),
                        env("S3_ACCESS_KEY"),
                        env("S3_SECRET_KEY"),
                        &api_protocol,
                    ),
                    proxy: None,
                }
            }
            (Env::Production, Replica::ReadOnlyMirror) => {
                // Read-only mirrors don't need access key or secret key,
                // but they might have them. Definitely need bucket though.
                Uploader::S3 {
                    bucket: s3::Bucket::new(
                        env("S3_BUCKET"),
                        env::var("S3_REGION").ok(),
                        env::var("S3_ACCESS_KEY").unwrap_or_default(),
                        env::var("S3_SECRET_KEY").unwrap_or_default(),
                        &api_protocol,
                    ),
                    proxy: None,
                }
            }
            _ => {
                if env::var("S3_BUCKET").is_ok() {
                    println!("Using S3 uploader");
                    Uploader::S3 {
                        bucket: s3::Bucket::new(
                            env("S3_BUCKET"),
                            env::var("S3_REGION").ok(),
                            env::var("S3_ACCESS_KEY").unwrap_or_default(),
                            env::var("S3_SECRET_KEY").unwrap_or_default(),
                            &api_protocol,
                        ),
                        proxy: None,
                    }
                } else {
                    println!("Using local uploader, crate files will be in the dist directory");
                    Uploader::Local
                }
            }
        };
        Config {
            uploader: uploader,
            session_key: env("SESSION_KEY"),
            git_repo_checkout: checkout,
            gh_client_id: env("GH_CLIENT_ID"),
            gh_client_secret: env("GH_CLIENT_SECRET"),
            db_url: env("DATABASE_URL"),
            env: cargo_env,
            max_upload_size: 10 * 1024 * 1024,
            mirror: mirror,
            api_protocol: api_protocol,
        }
    }
}
