use crate::env;
use anyhow::Context;
use futures_util::{StreamExt, TryStreamExt};
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::path::Path;
use object_store::{ObjectStore, Result};
use secrecy::{ExposeSecret, SecretString};
use std::fs;
use std::path::PathBuf;

const PREFIX_CRATES: &str = "crates";
const PREFIX_READMES: &str = "readmes";
const DEFAULT_REGION: &str = "us-west-1";

#[derive(Debug)]
pub enum StorageConfig {
    S3(S3Config),
    LocalFileSystem { path: PathBuf },
}

#[derive(Debug)]
pub struct S3Config {
    bucket: String,
    region: Option<String>,
    access_key: String,
    secret_key: SecretString,
}

impl StorageConfig {
    pub fn from_environment() -> Self {
        if let Ok(bucket) = dotenvy::var("S3_BUCKET") {
            let region = dotenvy::var("S3_REGION").ok();
            let access_key = env("AWS_ACCESS_KEY");
            let secret_key = env("AWS_SECRET_KEY").into();
            let s3 = S3Config {
                bucket,
                region,
                access_key,
                secret_key,
            };

            return Self::S3(s3);
        }

        let current_dir = std::env::current_dir()
            .context("Failed to read the current directory")
            .unwrap();

        let path = current_dir.join("local_uploads");

        Self::LocalFileSystem { path }
    }
}

pub struct Storage {
    store: Box<dyn ObjectStore>,
}

impl Storage {
    pub fn from_environment() -> Self {
        Self::from_config(&StorageConfig::from_environment())
    }

    pub fn from_config(config: &StorageConfig) -> Self {
        match config {
            StorageConfig::S3(s3) => {
                let s3 = AmazonS3Builder::new()
                    .with_region(s3.region.as_deref().unwrap_or(DEFAULT_REGION))
                    .with_bucket_name(&s3.bucket)
                    .with_access_key_id(&s3.access_key)
                    .with_secret_access_key(s3.secret_key.expose_secret())
                    .build()
                    .context("Failed to initialize S3 code")
                    .unwrap();

                let store = Box::new(s3);
                Self { store }
            }

            StorageConfig::LocalFileSystem { path } => {
                fs::create_dir_all(path)
                    .context("Failed to create `local_uploads` directory")
                    .unwrap();

                warn!(?path, "Using local file system for file storage");
                let local = LocalFileSystem::new_with_prefix(path)
                    .context("Failed to initialize local file system storage")
                    .unwrap();

                let store = Box::new(local);
                Self { store }
            }
        }
    }

    #[instrument(skip(self))]
    pub async fn delete_all_crate_files(&self, name: &str) -> Result<()> {
        let prefix = format!("{PREFIX_CRATES}/{name}").into();
        self.delete_all_with_prefix(&prefix).await
    }

    #[instrument(skip(self))]
    pub async fn delete_all_readmes(&self, name: &str) -> Result<()> {
        let prefix = format!("{PREFIX_READMES}/{name}").into();
        self.delete_all_with_prefix(&prefix).await
    }

    #[instrument(skip(self))]
    pub async fn delete_crate_file(&self, name: &str, version: &str) -> Result<()> {
        let path = format!("{PREFIX_CRATES}/{name}/{name}-{version}.crate").into();
        self.store.delete(&path).await
    }

    #[instrument(skip(self))]
    pub async fn delete_readme(&self, name: &str, version: &str) -> Result<()> {
        let path = format!("{PREFIX_READMES}/{name}/{name}-{version}.html").into();
        self.store.delete(&path).await
    }

    async fn delete_all_with_prefix(&self, prefix: &Path) -> Result<()> {
        let objects = self.store.list(Some(prefix)).await?;
        let locations = objects.map(|meta| meta.map(|m| m.location)).boxed();

        self.store
            .delete_stream(locations)
            .try_collect::<Vec<_>>()
            .await?;

        Ok(())
    }
}
