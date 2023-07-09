use crate::env;
use anyhow::Context;
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::ObjectStore;
use std::fs;

const DEFAULT_REGION: &str = "us-west-1";

pub fn from_environment() -> Box<dyn ObjectStore> {
    if let Ok(bucket) = dotenvy::var("S3_BUCKET") {
        let region = dotenvy::var("S3_REGION").unwrap_or(DEFAULT_REGION.to_string());
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

        return Box::new(s3);
    }

    let current_dir = std::env::current_dir()
        .context("Failed to read the current directory")
        .unwrap();

    let path = current_dir.join("local_uploads");

    fs::create_dir_all(&path)
        .context("Failed to create `local_uploads` directory")
        .unwrap();

    warn!(?path, "Using local file system for file storage");
    let local = LocalFileSystem::new_with_prefix(path)
        .context("Failed to initialize local file system storage")
        .unwrap();

    Box::new(local)
}
