use crate::env;
use anyhow::Context;
use object_store::aws::AmazonS3Builder;
use object_store::ObjectStore;

const DEFAULT_REGION: &str = "us-west-1";

pub fn from_environment() -> Box<dyn ObjectStore> {
    let region = dotenvy::var("S3_REGION").unwrap_or(DEFAULT_REGION.to_string());
    let bucket = env("S3_BUCKET");
    let access_key = env("AWS_ACCESS_KEY");
    let secret_key = env("AWS_SECRET_KEY");

    let s3 = AmazonS3Builder::new()
        .with_region(region)
        .with_bucket_name(bucket)
        .with_access_key_id(access_key)
        .with_secret_access_key(secret_key)
        .build()
        .context("Failed to initialize S3 code")
        .unwrap();

    Box::new(s3)
}
