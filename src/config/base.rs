//! Base configuration options
//!
//! - `HEROKU`: Is this instance of crates_io:: currently running on Heroku.
//! - `S3_BUCKET`: The S3 bucket used to store crate files. If not present during development,
//!    crates_io:: will fall back to a local uploader.
//! - `S3_REGION`: The region in which the bucket was created. Optional if US standard.
//! - `AWS_ACCESS_KEY`: The access key to interact with S3.
//! - `AWS_SECRET_KEY`: The secret key to interact with S3.
//! - `S3_CDN`: Optional CDN configuration for building public facing URLs.

use crate::{env, uploaders::Uploader, Env};

pub struct Base {
    pub env: Env,
    uploader: Uploader,
}

impl Base {
    pub fn from_environment() -> Self {
        let heroku = dotenvy::var("HEROKU").is_ok();
        let env = if heroku {
            Env::Production
        } else {
            Env::Development
        };

        let uploader = match env {
            Env::Production => {
                // `env` panics if these vars are not set, and in production for a primary instance,
                // that's what we want since we don't want to be able to start the server if the
                // server doesn't know where to upload crates.
                Self::s3_panic_if_missing_keys()
            }
            // In Development mode, either running as a primary instance or a read-only mirror
            _ => {
                if dotenvy::var("S3_BUCKET").is_ok() {
                    // If we've set the `S3_BUCKET` variable to any value, use all of the values
                    // for the related S3 environment variables and configure the app to upload to
                    // and read from S3 like production does. All values except for bucket are
                    // optional, like production read-only mirrors.
                    info!("Using S3 uploader");
                    Self::s3_maybe_read_only()
                } else {
                    // If we don't set the `S3_BUCKET` variable, we'll use a development-only
                    // uploader that makes it possible to run and publish to a locally-running
                    // crates.io instance without needing to set up an account and a bucket in S3.
                    info!(
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
            bucket: Box::new(s3::Bucket::new(
                String::from("alexcrichton-test"),
                s3::Region::Default,
                dotenvy::var("AWS_ACCESS_KEY").unwrap_or_default(),
                dotenvy::var("AWS_SECRET_KEY").unwrap_or_default(),
                // When testing we route all API traffic over HTTP so we can
                // sniff/record it, but everywhere else we use https
                "http",
            )),
            index_bucket: Some(Box::new(s3::Bucket::new(
                String::from("alexcrichton-test"),
                s3::Region::Default,
                dotenvy::var("AWS_ACCESS_KEY").unwrap_or_default(),
                dotenvy::var("AWS_SECRET_KEY").unwrap_or_default(),
                // When testing we route all API traffic over HTTP so we can
                // sniff/record it, but everywhere else we use https
                "http",
            ))),
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
        let index_bucket = match dotenvy::var("S3_INDEX_BUCKET") {
            Ok(name) => Some(Box::new(s3::Bucket::new(
                name,
                dotenvy::var("S3_INDEX_REGION")
                    .map_or_else(|_err| s3::Region::Default, s3::Region::Region),
                env("AWS_ACCESS_KEY"),
                env("AWS_SECRET_KEY"),
                "https",
            ))),
            Err(_) => None,
        };
        Uploader::S3 {
            bucket: Box::new(s3::Bucket::new(
                env("S3_BUCKET"),
                dotenvy::var("S3_REGION")
                    .map_or_else(|_err| s3::Region::Default, s3::Region::Region),
                env("AWS_ACCESS_KEY"),
                env("AWS_SECRET_KEY"),
                "https",
            )),
            index_bucket,
            cdn: dotenvy::var("S3_CDN").ok(),
        }
    }

    fn s3_maybe_read_only() -> Uploader {
        let index_bucket = match dotenvy::var("S3_INDEX_BUCKET") {
            Ok(name) => Some(Box::new(s3::Bucket::new(
                name,
                dotenvy::var("S3_INDEX_REGION")
                    .map_or_else(|_err| s3::Region::Default, s3::Region::Region),
                dotenvy::var("AWS_ACCESS_KEY").unwrap_or_default(),
                dotenvy::var("AWS_SECRET_KEY").unwrap_or_default(),
                "https",
            ))),
            Err(_) => None,
        };
        Uploader::S3 {
            bucket: Box::new(s3::Bucket::new(
                env("S3_BUCKET"),
                dotenvy::var("S3_REGION")
                    .map_or_else(|_err| s3::Region::Default, s3::Region::Region),
                dotenvy::var("AWS_ACCESS_KEY").unwrap_or_default(),
                dotenvy::var("AWS_SECRET_KEY").unwrap_or_default(),
                "https",
            )),
            index_bucket,
            cdn: dotenvy::var("S3_CDN").ok(),
        }
    }
}
