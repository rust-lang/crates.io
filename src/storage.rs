use crate::env;
use anyhow::Context;
use object_store::aws::{AmazonS3, AmazonS3Builder};

pub fn from_environment() -> AmazonS3 {
    let region = dotenvy::var("S3_REGION").unwrap_or("us-west-1".to_string());
    let bucket = env("S3_BUCKET");
    let access_key = env("AWS_ACCESS_KEY");
    let secret_key = env("AWS_SECRET_KEY");

    AmazonS3Builder::new()
        .with_region(region)
        .with_bucket_name(bucket)
        .with_access_key_id(access_key)
        .with_secret_access_key(secret_key)
        .build()
        .context("Failed to initialize S3 code")
        .unwrap()
}
