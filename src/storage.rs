use anyhow::Context;
use crates_io_env_vars::required_var;
use futures_util::{StreamExt, TryStreamExt};
use hyper::body::Bytes;
use object_store::aws::{AmazonS3, AmazonS3Builder};
use object_store::local::LocalFileSystem;
use object_store::memory::InMemory;
use object_store::path::Path;
use object_store::prefix::PrefixStore;
use object_store::{Attribute, Attributes, ClientOptions, ObjectStore, PutPayload, Result};
use secrecy::{ExposeSecret, SecretString};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

const PREFIX_CRATES: &str = "crates";
const PREFIX_READMES: &str = "readmes";
const DEFAULT_REGION: &str = "us-west-1";
const CONTENT_TYPE_CRATE: &str = "application/gzip";
const CONTENT_TYPE_GZIP: &str = "application/gzip";
const CONTENT_TYPE_ZIP: &str = "application/zip";
const CONTENT_TYPE_INDEX: &str = "text/plain";
const CONTENT_TYPE_README: &str = "text/html";
const CACHE_CONTROL_IMMUTABLE: &str = "public,max-age=31536000,immutable";
const CACHE_CONTROL_INDEX: &str = "public,max-age=600";
const CACHE_CONTROL_README: &str = "public,max-age=604800";

type StdPath = std::path::Path;

#[derive(Debug)]
pub struct StorageConfig {
    backend: StorageBackend,
    pub cdn_prefix: Option<String>,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum StorageBackend {
    S3 { default: S3Config, index: S3Config },
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
    pub fn in_memory() -> Self {
        Self {
            backend: StorageBackend::InMemory,
            cdn_prefix: None,
        }
    }

    pub fn from_environment() -> Self {
        if let Ok(bucket) = dotenvy::var("S3_BUCKET") {
            let region = dotenvy::var("S3_REGION").ok();
            let cdn_prefix = dotenvy::var("S3_CDN").ok();

            let index_bucket = required_var("S3_INDEX_BUCKET").unwrap();
            let index_region = dotenvy::var("S3_INDEX_REGION").ok();

            let access_key = required_var("AWS_ACCESS_KEY").unwrap();
            let secret_key: SecretString = required_var("AWS_SECRET_KEY").unwrap().into();

            let default = S3Config {
                bucket,
                region,
                access_key: access_key.clone(),
                secret_key: secret_key.clone(),
            };

            let index = S3Config {
                bucket: index_bucket,
                region: index_region,
                access_key,
                secret_key,
            };

            let backend = StorageBackend::S3 { default, index };

            return Self {
                backend,
                cdn_prefix,
            };
        }

        let current_dir = std::env::current_dir()
            .context("Failed to read the current directory")
            .unwrap();

        let path = current_dir.join("local_uploads");

        let backend = StorageBackend::LocalFileSystem { path };

        Self {
            backend,
            cdn_prefix: None,
        }
    }
}

pub struct Storage {
    cdn_prefix: Option<String>,
    store: Arc<dyn ObjectStore>,
    index_store: Arc<dyn ObjectStore>,
    supports_attributes: bool,
}

impl Storage {
    pub fn from_environment() -> Self {
        Self::from_config(&StorageConfig::from_environment())
    }

    pub fn from_config(config: &StorageConfig) -> Self {
        let cdn_prefix = config.cdn_prefix.clone();

        match &config.backend {
            StorageBackend::S3 { default, index } => {
                let options = ClientOptions::default()
                    // The `BufWriter::new()` API currently does not allow
                    // specifying any file attributes, so we need to set the
                    // content type here instead for the database dump upload.
                    .with_content_type_for_suffix("gz", CONTENT_TYPE_GZIP)
                    .with_content_type_for_suffix("zip", CONTENT_TYPE_ZIP);

                let store = build_s3(default, options);

                let index_store = build_s3(index, Default::default());

                if cdn_prefix.is_none() {
                    panic!("Missing S3_CDN environment variable");
                }

                Self {
                    cdn_prefix,
                    store: Arc::new(store),
                    index_store: Arc::new(index_store),
                    supports_attributes: true,
                }
            }

            StorageBackend::LocalFileSystem { path } => {
                warn!(?path, "Using local file system for file storage");

                let index_path = path.join("index");

                fs::create_dir_all(&index_path)
                    .context("Failed to create file storage directories")
                    .unwrap();

                let local = LocalFileSystem::new_with_prefix(path)
                    .context("Failed to initialize local file system storage")
                    .unwrap();

                let local_index = LocalFileSystem::new_with_prefix(index_path)
                    .context("Failed to initialize local file system storage")
                    .unwrap();

                let store: Arc<dyn ObjectStore> = Arc::new(local);
                let index_store: Arc<dyn ObjectStore> = Arc::new(local_index);

                Self {
                    cdn_prefix,
                    store,
                    index_store,
                    supports_attributes: false,
                }
            }

            StorageBackend::InMemory => {
                warn!("Using in-memory file storage");
                let store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());

                Self {
                    cdn_prefix,
                    store: store.clone(),
                    index_store: Arc::new(PrefixStore::new(store, "index")),
                    supports_attributes: true,
                }
            }
        }
    }

    /// Returns the URL of an uploaded crate's version archive.
    ///
    /// The function doesn't check for the existence of the file.
    pub fn crate_location(&self, name: &str, version: &str) -> String {
        apply_cdn_prefix(&self.cdn_prefix, &crate_file_path(name, version)).replace('+', "%2B")
    }

    /// Returns the URL of an uploaded crate's version readme.
    ///
    /// The function doesn't check for the existence of the file.
    pub fn readme_location(&self, name: &str, version: &str) -> String {
        apply_cdn_prefix(&self.cdn_prefix, &readme_path(name, version)).replace('+', "%2B")
    }

    /// Returns the URL of an uploaded RSS feed.
    pub fn feed_url(&self, feed_id: &FeedId) -> String {
        apply_cdn_prefix(&self.cdn_prefix, &feed_id.into()).replace('+', "%2B")
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

    #[instrument(skip(self, bytes))]
    pub async fn upload_crate_file(&self, name: &str, version: &str, bytes: Bytes) -> Result<()> {
        let path = crate_file_path(name, version);
        let attributes = self.attrs([
            (Attribute::ContentType, CONTENT_TYPE_CRATE),
            (Attribute::CacheControl, CACHE_CONTROL_IMMUTABLE),
        ]);
        let opts = attributes.into();
        self.store.put_opts(&path, bytes.into(), opts).await?;
        Ok(())
    }

    #[instrument(skip(self, bytes))]
    pub async fn upload_readme(&self, name: &str, version: &str, bytes: Bytes) -> Result<()> {
        let path = readme_path(name, version);
        let attributes = self.attrs([
            (Attribute::ContentType, CONTENT_TYPE_README),
            (Attribute::CacheControl, CACHE_CONTROL_README),
        ]);
        let opts = attributes.into();
        self.store.put_opts(&path, bytes.into(), opts).await?;
        Ok(())
    }

    #[instrument(skip(self, channel))]
    pub async fn upload_feed(
        &self,
        feed_id: &FeedId,
        channel: &rss::Channel,
    ) -> anyhow::Result<()> {
        let path = feed_id.into();

        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        channel.pretty_write_to(&mut cursor, b' ', 4)?;
        let payload = PutPayload::from_bytes(buffer.into());

        let attributes = self.attrs([(Attribute::ContentType, "text/xml; charset=UTF-8")]);
        let opts = attributes.into();
        self.store.put_opts(&path, payload, opts).await?;
        Ok(())
    }

    #[instrument(skip(self, content))]
    pub async fn sync_index(&self, name: &str, content: Option<String>) -> Result<()> {
        let path = crates_io_index::Repository::relative_index_file_for_url(name).into();
        if let Some(content) = content {
            let attributes = self.attrs([
                (Attribute::ContentType, CONTENT_TYPE_INDEX),
                (Attribute::CacheControl, CACHE_CONTROL_INDEX),
            ]);
            let payload = content.into();
            let opts = attributes.into();
            self.index_store.put_opts(&path, payload, opts).await?;
        } else {
            self.index_store.delete(&path).await?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn upload_db_dump(&self, target: &str, local_path: &StdPath) -> anyhow::Result<()> {
        let store = self.store.clone();

        // Open the local tarball file
        let mut local_file = File::open(local_path).await?;

        // Set up a multipart upload
        let path = target.into();
        let mut writer = object_store::buffered::BufWriter::new(store, path);

        // Upload file contents
        if let Err(error) = tokio::io::copy(&mut local_file, &mut writer).await {
            // Abort the upload if something failed
            writer.abort().await?;
            return Err(error.into());
        }

        // ... or finalize upload
        writer.shutdown().await?;

        Ok(())
    }

    /// This should only be used for assertions in the test suite!
    pub fn as_inner(&self) -> &dyn ObjectStore {
        &self.store
    }

    async fn delete_all_with_prefix(&self, prefix: &Path) -> Result<()> {
        let objects = self.store.list(Some(prefix));
        let locations = objects.map(|meta| meta.map(|m| m.location)).boxed();

        self.store
            .delete_stream(locations)
            .try_collect::<Vec<_>>()
            .await?;

        Ok(())
    }

    fn attrs(&self, slice: impl IntoIterator<Item = (Attribute, &'static str)>) -> Attributes {
        if self.supports_attributes {
            Attributes::from_iter(slice)
        } else {
            Attributes::new()
        }
    }
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

fn apply_cdn_prefix(cdn_prefix: &Option<String>, path: &Path) -> String {
    match cdn_prefix {
        Some(cdn_prefix) if !cdn_prefix.starts_with("https://") => {
            format!("https://{cdn_prefix}/{path}")
        }
        Some(cdn_prefix) => format!("{cdn_prefix}/{path}"),
        None => format!("/{path}"),
    }
}

#[derive(Debug)]
pub enum FeedId {
    Crate { name: String },
    Crates,
    Updates,
}

impl From<&FeedId> for Path {
    fn from(feed_id: &FeedId) -> Path {
        match feed_id {
            FeedId::Crate { name } => format!("rss/crates/{name}.xml").into(),
            FeedId::Crates => "rss/crates.xml".into(),
            FeedId::Updates => "rss/updates.xml".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::body::Bytes;
    use tempfile::NamedTempFile;

    pub async fn prepare() -> Storage {
        let storage = Storage::from_config(&StorageConfig::in_memory());

        let files_to_create = vec![
            "crates/bar/bar-2.0.0.crate",
            "crates/foo/foo-1.0.0.crate",
            "crates/foo/foo-1.2.3.crate",
            "readmes/bar/bar-2.0.0.html",
            "readmes/foo/foo-1.0.0.html",
            "readmes/foo/foo-1.2.3.html",
        ];
        for path in files_to_create {
            let payload = Bytes::new().into();
            storage.store.put(&path.into(), payload).await.unwrap();
        }

        storage
    }

    pub async fn stored_files(store: &dyn ObjectStore) -> Vec<String> {
        let stream = store.list(None);
        let list = stream.try_collect::<Vec<_>>().await.unwrap();

        list.into_iter()
            .map(|meta| meta.location.to_string())
            .collect()
    }

    #[test]
    fn locations() {
        let mut config = StorageConfig::in_memory();
        config.cdn_prefix = Some("static.crates.io".to_string());

        let storage = Storage::from_config(&config);

        let crate_tests = vec![
            ("foo", "1.2.3", "https://static.crates.io/crates/foo/foo-1.2.3.crate"),
            (
                "some-long-crate-name",
                "42.0.5-beta.1+foo",
                "https://static.crates.io/crates/some-long-crate-name/some-long-crate-name-42.0.5-beta.1%2Bfoo.crate",
            ),
        ];
        for (name, version, expected) in crate_tests {
            assert_eq!(storage.crate_location(name, version), expected);
        }

        let readme_tests = vec![
            ("foo", "1.2.3", "https://static.crates.io/readmes/foo/foo-1.2.3.html"),
            (
                "some-long-crate-name",
                "42.0.5-beta.1+foo",
                "https://static.crates.io/readmes/some-long-crate-name/some-long-crate-name-42.0.5-beta.1%2Bfoo.html",
            ),
        ];
        for (name, version, expected) in readme_tests {
            assert_eq!(storage.readme_location(name, version), expected);
        }
    }

    #[test]
    fn cdn_prefix() {
        assert_eq!(apply_cdn_prefix(&None, &"foo".into()), "/foo");
        assert_eq!(
            apply_cdn_prefix(&Some("static.crates.io".to_string()), &"foo".into()),
            "https://static.crates.io/foo"
        );
        assert_eq!(
            apply_cdn_prefix(
                &Some("https://fastly-static.crates.io".to_string()),
                &"foo".into()
            ),
            "https://fastly-static.crates.io/foo"
        );

        assert_eq!(
            apply_cdn_prefix(&Some("static.crates.io".to_string()), &"/foo/bar".into()),
            "https://static.crates.io/foo/bar"
        );

        assert_eq!(
            apply_cdn_prefix(&Some("static.crates.io/".to_string()), &"/foo/bar".into()),
            "https://static.crates.io//foo/bar"
        );
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
        let s = Storage::from_config(&StorageConfig::in_memory());

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
            "crates/foo/foo-2.0.0+foo.crate",
        ];
        assert_eq!(stored_files(&s.store).await, expected_files);
    }

    #[tokio::test]
    async fn upload_readme() {
        let s = Storage::from_config(&StorageConfig::in_memory());

        let bytes = Bytes::from_static(b"hello world");
        s.upload_readme("foo", "1.2.3", bytes.clone())
            .await
            .unwrap();

        let expected_files = vec!["readmes/foo/foo-1.2.3.html"];
        assert_eq!(stored_files(&s.store).await, expected_files);

        s.upload_readme("foo", "2.0.0+foo", bytes).await.unwrap();

        let expected_files = vec![
            "readmes/foo/foo-1.2.3.html",
            "readmes/foo/foo-2.0.0+foo.html",
        ];
        assert_eq!(stored_files(&s.store).await, expected_files);
    }

    #[tokio::test]
    async fn sync_index() {
        let s = Storage::from_config(&StorageConfig::in_memory());

        assert!(stored_files(&s.store).await.is_empty());

        let content = "foo".to_string();
        s.sync_index("foo", Some(content)).await.unwrap();

        let expected_files = vec!["index/3/f/foo"];
        assert_eq!(stored_files(&s.store).await, expected_files);

        s.sync_index("foo", None).await.unwrap();

        assert!(stored_files(&s.store).await.is_empty());
    }

    #[tokio::test]
    async fn upload_db_dump() {
        let s = Storage::from_config(&StorageConfig::in_memory());

        assert!(stored_files(&s.store).await.is_empty());

        let target = "db-dump.tar.gz";
        let file = NamedTempFile::new().unwrap();
        s.upload_db_dump(target, file.path()).await.unwrap();

        let expected_files = vec![target];
        assert_eq!(stored_files(&s.store).await, expected_files);
    }
}
