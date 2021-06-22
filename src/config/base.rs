//! Base configuration options
//!
//! - `HEROKU`: Is this instance of cargo_registry currently running on Heroku.
//! - `MIRROR`: (deprecated) Is this instance of cargo_registry a mirror of crates.io.
//! - `S3_BUCKET`: The S3 bucket used to store crate files. If not present during development,
//!    cargo_registry will fall back to a local uploader.
//! - `S3_REGION`: The region in which the bucket was created. Optional if US standard.
//! - `S3_ACCESS_KEY`: The access key to interact with S3. Optional if running a mirror.
//! - `S3_SECRET_KEY`: The secret key to interact with S3. Optional if running a mirror.
//! - `S3_CDN`: Optional CDN configuration for building public facing URLs.

use crate::{env, uploaders::Uploader, Env, Replica};

pub struct Base {
    pub(super) env: Env,
    uploader: Uploader,
}

impl Base {
    pub fn from_environment() -> Self {
        let mirror = if dotenv::var("MIRROR").is_ok() {
            Replica::ReadOnlyMirror
        } else {
            Replica::Primary
        };
        let heroku = dotenv::var("HEROKU").is_ok();
        let env = if heroku {
            Env::Production
        } else {
            Env::Development
        };

        let uploader = match (env, mirror) {
            (Env::Production, Replica::Primary) => {
                // `env` panics if these vars are not set, and in production for a primary instance,
                // that's what we want since we don't want to be able to start the server if the
                // server doesn't know where to upload crates.
                Self::s3_panic_if_missing_keys()
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
                Self::s3_maybe_read_only()
            }
            // In Development mode, either running as a primary instance or a read-only mirror
            _ => {
                if dotenv::var("S3_BUCKET").is_ok() {
                    // If we've set the `S3_BUCKET` variable to any value, use all of the values
                    // for the related S3 environment variables and configure the app to upload to
                    // and read from S3 like production does. All values except for bucket are
                    // optional, like production read-only mirrors.
                    println!("Using S3 uploader");
                    Self::s3_maybe_read_only()
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

        Self { env, uploader }
    }

    pub fn test() -> Self {
        let uploader = Uploader::S3 {
            bucket: s3::Bucket::new(
                String::from("alexcrichton-test"),
                None,
                dotenv::var("S3_ACCESS_KEY").unwrap_or_default(),
                dotenv::var("S3_SECRET_KEY").unwrap_or_default(),
                // When testing we route all API traffic over HTTP so we can
                // sniff/record it, but everywhere else we use https
                "http",
            ),
            cdn: None,
        };
        Self {
            env: Env::Test,
            uploader,
        }
    }

    pub fn uploader(&self) -> &Uploader {
        &self.uploader
    }

    fn s3_panic_if_missing_keys() -> Uploader {
        Uploader::S3 {
            bucket: s3::Bucket::new(
                env("S3_BUCKET"),
                dotenv::var("S3_REGION").ok(),
                env("S3_ACCESS_KEY"),
                env("S3_SECRET_KEY"),
                "https",
            ),
            cdn: dotenv::var("S3_CDN").ok(),
        }
    }

    fn s3_maybe_read_only() -> Uploader {
        Uploader::S3 {
            bucket: s3::Bucket::new(
                env("S3_BUCKET"),
                dotenv::var("S3_REGION").ok(),
                dotenv::var("S3_ACCESS_KEY").unwrap_or_default(),
                dotenv::var("S3_SECRET_KEY").unwrap_or_default(),
                "https",
            ),
            cdn: dotenv::var("S3_CDN").ok(),
        }
    }
}
