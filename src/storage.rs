mod arc_store;

use crate::env;
use crate::storage::arc_store::ArcStore;
use anyhow::Context;
use futures_util::{StreamExt, TryStreamExt};
use http::header::CACHE_CONTROL;
use http::{HeaderMap, HeaderValue};
use hyper::body::Bytes;
use object_store::aws::{AmazonS3, AmazonS3Builder};
use object_store::local::LocalFileSystem;
use object_store::memory::InMemory;
use object_store::path::Path;
use object_store::{ClientOptions, ObjectStore, Result};
use secrecy::{ExposeSecret, SecretString};
use std::fs;
use std::path::PathBuf;

const PREFIX_CRATES: &str = "crates";
const PREFIX_READMES: &str = "readmes";
const DEFAULT_REGION: &str = "us-west-1";
const CONTENT_TYPE_CRATE: &str = "application/gzip";
const CONTENT_TYPE_README: &str = "text/html";
const CACHE_CONTROL_IMMUTABLE: &str = "public,max-age=31536000,immutable";
const CACHE_CONTROL_README: &str = "public,max-age=604800";

#[derive(Debug)]
pub enum StorageConfig {
    S3(S3Config),
    LocalFileSystem { path: PathBuf },
    InMemory,
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
    crate_upload_store: Box<dyn ObjectStore>,
    readme_upload_store: Box<dyn ObjectStore>,
}

impl Storage {
    pub fn from_environment() -> Self {
        Self::from_config(&StorageConfig::from_environment())
    }

    pub fn from_config(config: &StorageConfig) -> Self {
        match config {
            StorageConfig::S3(s3) => {
                let options = ClientOptions::default();
                let store = build_s3(s3, options);

                let options = client_options(CONTENT_TYPE_CRATE, CACHE_CONTROL_IMMUTABLE);
                let crate_upload_store = build_s3(s3, options);

                let options = client_options(CONTENT_TYPE_README, CACHE_CONTROL_README);
                let readme_upload_store = build_s3(s3, options);

                Self {
                    store: Box::new(store),
                    crate_upload_store: Box::new(crate_upload_store),
                    readme_upload_store: Box::new(readme_upload_store),
                }
            }

            StorageConfig::LocalFileSystem { path } => {
                fs::create_dir_all(path)
                    .context("Failed to create `local_uploads` directory")
                    .unwrap();

                warn!(?path, "Using local file system for file storage");
                let local = LocalFileSystem::new_with_prefix(path)
                    .context("Failed to initialize local file system storage")
                    .unwrap();

                let store = ArcStore::new(local);
                Self {
                    store: Box::new(store.clone()),
                    crate_upload_store: Box::new(store.clone()),
                    readme_upload_store: Box::new(store),
                }
            }

            StorageConfig::InMemory => {
                warn!("Using in-memory file storage");
                let store = ArcStore::new(InMemory::new());
                Self {
                    store: Box::new(store.clone()),
                    crate_upload_store: Box::new(store.clone()),
                    readme_upload_store: Box::new(store),
                }
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
        let path = crate_file_path(name, version);
        self.store.delete(&path).await
    }

    #[instrument(skip(self))]
    pub async fn delete_readme(&self, name: &str, version: &str) -> Result<()> {
        let path = readme_path(name, version);
        self.store.delete(&path).await
    }

    #[instrument(skip(self))]
    pub async fn upload_crate_file(&self, name: &str, version: &str, bytes: Bytes) -> Result<()> {
        if version.contains('+') {
            let version = version.replace('+', " ");
            let path = crate_file_path(name, &version);
            self.crate_upload_store.put(&path, bytes.clone()).await?
        }

        let path = crate_file_path(name, version);
        self.crate_upload_store.put(&path, bytes).await
    }

    #[instrument(skip(self))]
    pub async fn upload_readme(&self, name: &str, version: &str, bytes: Bytes) -> Result<()> {
        if version.contains('+') {
            let version = version.replace('+', " ");
            let path = readme_path(name, &version);
            self.readme_upload_store.put(&path, bytes.clone()).await?
        }

        let path = readme_path(name, version);
        self.readme_upload_store.put(&path, bytes).await
    }

    /// This should only be used for assertions in the test suite!
    pub fn as_inner(&self) -> &dyn ObjectStore {
        &self.store
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

fn client_options(content_type: &str, cache_control: &'static str) -> ClientOptions {
    let mut headers = HeaderMap::new();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static(cache_control));

    ClientOptions::default()
        .with_default_content_type(content_type)
        .with_default_headers(headers)
}

fn build_s3(config: &S3Config, client_options: ClientOptions) -> AmazonS3 {
    AmazonS3Builder::new()
        .with_region(config.region.as_deref().unwrap_or(DEFAULT_REGION))
        .with_bucket_name(&config.bucket)
        .with_access_key_id(&config.access_key)
        .with_secret_access_key(config.secret_key.expose_secret())
        .with_client_options(client_options)
        .build()
        .context("Failed to initialize S3 code")
        .unwrap()
}

fn crate_file_path(name: &str, version: &str) -> Path {
    format!("{PREFIX_CRATES}/{name}/{name}-{version}.crate").into()
}

fn readme_path(name: &str, version: &str) -> Path {
    format!("{PREFIX_READMES}/{name}/{name}-{version}.html").into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::body::Bytes;

    pub async fn prepare() -> Storage {
        let storage = Storage::from_config(&StorageConfig::InMemory);

        let files_to_create = vec![
            "crates/bar/bar-2.0.0.crate",
            "crates/foo/foo-1.0.0.crate",
            "crates/foo/foo-1.2.3.crate",
            "readmes/bar/bar-2.0.0.html",
            "readmes/foo/foo-1.0.0.html",
            "readmes/foo/foo-1.2.3.html",
        ];
        for path in files_to_create {
            storage.store.put(&path.into(), Bytes::new()).await.unwrap();
        }

        storage
    }

    pub async fn stored_files(store: &dyn ObjectStore) -> Vec<String> {
        let stream = store.list(None).await.unwrap();
        let list = stream.try_collect::<Vec<_>>().await.unwrap();

        list.into_iter()
            .map(|meta| meta.location.to_string())
            .collect()
    }

    #[tokio::test]
    async fn delete_all_crate_files() {
        let storage = prepare().await;

        storage.delete_all_crate_files("foo").await.unwrap();

        let expected_files = vec![
            "crates/bar/bar-2.0.0.crate",
            "readmes/bar/bar-2.0.0.html",
            "readmes/foo/foo-1.0.0.html",
            "readmes/foo/foo-1.2.3.html",
        ];
        assert_eq!(stored_files(&storage.store).await, expected_files);
    }

    #[tokio::test]
    async fn delete_all_readmes() {
        let storage = prepare().await;

        storage.delete_all_readmes("foo").await.unwrap();

        let expected_files = vec![
            "crates/bar/bar-2.0.0.crate",
            "crates/foo/foo-1.0.0.crate",
            "crates/foo/foo-1.2.3.crate",
            "readmes/bar/bar-2.0.0.html",
        ];
        assert_eq!(stored_files(&storage.store).await, expected_files);
    }

    #[tokio::test]
    async fn delete_crate_file() {
        let storage = prepare().await;

        storage.delete_crate_file("foo", "1.2.3").await.unwrap();

        let expected_files = vec![
            "crates/bar/bar-2.0.0.crate",
            "crates/foo/foo-1.0.0.crate",
            "readmes/bar/bar-2.0.0.html",
            "readmes/foo/foo-1.0.0.html",
            "readmes/foo/foo-1.2.3.html",
        ];
        assert_eq!(stored_files(&storage.store).await, expected_files);
    }

    #[tokio::test]
    async fn delete_readme() {
        let storage = prepare().await;

        storage.delete_readme("foo", "1.2.3").await.unwrap();

        let expected_files = vec![
            "crates/bar/bar-2.0.0.crate",
            "crates/foo/foo-1.0.0.crate",
            "crates/foo/foo-1.2.3.crate",
            "readmes/bar/bar-2.0.0.html",
            "readmes/foo/foo-1.0.0.html",
        ];
        assert_eq!(stored_files(&storage.store).await, expected_files);
    }

    #[tokio::test]
    async fn upload_crate_file() {
        let s = Storage::from_config(&StorageConfig::InMemory);

        s.upload_crate_file("foo", "1.2.3", Bytes::new())
            .await
            .unwrap();

        let expected_files = vec!["crates/foo/foo-1.2.3.crate"];
        assert_eq!(stored_files(&s.store).await, expected_files);

        s.upload_crate_file("foo", "2.0.0+foo", Bytes::new())
            .await
            .unwrap();

        let expected_files = vec![
            "crates/foo/foo-1.2.3.crate",
            "crates/foo/foo-2.0.0 foo.crate",
            "crates/foo/foo-2.0.0+foo.crate",
        ];
        assert_eq!(stored_files(&s.store).await, expected_files);
    }

    #[tokio::test]
    async fn upload_readme() {
        let s = Storage::from_config(&StorageConfig::InMemory);

        let bytes = Bytes::from_static(b"hello world");
        s.upload_readme("foo", "1.2.3", bytes.clone())
            .await
            .unwrap();

        let expected_files = vec!["readmes/foo/foo-1.2.3.html"];
        assert_eq!(stored_files(&s.store).await, expected_files);

        s.upload_readme("foo", "2.0.0+foo", bytes).await.unwrap();

        let expected_files = vec![
            "readmes/foo/foo-1.2.3.html",
            "readmes/foo/foo-2.0.0 foo.html",
            "readmes/foo/foo-2.0.0+foo.html",
        ];
        assert_eq!(stored_files(&s.store).await, expected_files);
    }
}
