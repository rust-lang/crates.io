use anyhow::Context;
use crates_io_env_vars::required_var;
use futures_util::stream::BoxStream;
use futures_util::{StreamExt, TryStreamExt};
use hyper::body::Bytes;
use object_store::aws::{AmazonS3, AmazonS3Builder};
use object_store::local::LocalFileSystem;
use object_store::memory::InMemory;
use object_store::path::Path;
use object_store::prefix::PrefixStore;
use object_store::{
    Attribute, Attributes, ClientOptions, ObjectStore, ObjectStoreExt, PutPayload, Result,
};
use secrecy::{ExposeSecret, SecretString};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWriteExt};
use tracing::{instrument, warn};

const PREFIX_CRATES: &str = "crates";
const PREFIX_READMES: &str = "readmes";
const PREFIX_OG_IMAGES: &str = "og-images";
const DEFAULT_REGION: &str = "us-west-1";
const CACHE_CONTROL_IMMUTABLE: &str = "public,max-age=31536000,immutable";
const CACHE_CONTROL_INDEX: &str = "public,max-age=600";
const CACHE_CONTROL_README: &str = "public,max-age=604800";
const CACHE_CONTROL_OG_IMAGE: &str = "public,max-age=86400";

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
    cdn_base: String,
    store: Arc<dyn ObjectStore>,
    index_store: Arc<dyn ObjectStore>,
    supports_attributes: bool,
}

impl Storage {
    pub fn from_environment() -> Self {
        Self::from_config(&StorageConfig::from_environment())
    }

    pub fn from_config(config: &StorageConfig) -> Self {
        let cdn_base = match config.cdn_prefix.as_deref() {
            Some(prefix) if prefix.starts_with("https://") => prefix.to_string(),
            Some(prefix) => format!("https://{prefix}"),
            None => String::new(),
        };

        match &config.backend {
            StorageBackend::S3 { default, index } => {
                let options = ClientOptions::default()
                    // Apply default content types for the version downloads archive
                    .with_content_type_for_suffix("html", "text/html")
                    .with_content_type_for_suffix("json", "application/json")
                    .with_content_type_for_suffix("csv", "text/csv")
                    // The `BufWriter::new()` API currently does not allow
                    // specifying any file attributes, so we need to set the
                    // content type here instead for the database dump upload.
                    .with_content_type_for_suffix("gz", "application/gzip")
                    .with_content_type_for_suffix("zip", "application/zip");

                let store = build_s3(default, options);

                let index_store = build_s3(index, Default::default());

                if config.cdn_prefix.is_none() {
                    panic!("Missing S3_CDN environment variable");
                }

                Self {
                    cdn_base,
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
                    cdn_base,
                    store,
                    index_store,
                    supports_attributes: false,
                }
            }

            StorageBackend::InMemory => {
                warn!("Using in-memory file storage");
                let store: Arc<dyn ObjectStore> = Arc::new(InMemory::new());

                Self {
                    cdn_base,
                    store: store.clone(),
                    index_store: Arc::new(PrefixStore::new(store, "index")),
                    supports_attributes: true,
                }
            }
        }
    }

    /// Returns the public URL of the file identified by `key`.
    ///
    /// The function doesn't check for the existence of the file.
    pub fn location(&self, key: &StorageKey<'_>) -> String {
        format!("{}/{}", self.cdn_base, key.cdn_path())
    }

    /// Returns the base URL that crate files are served from, e.g.
    /// `https://static.crates.io`.
    pub fn cdn_base(&self) -> &str {
        &self.cdn_base
    }

    /// Deletes all crate files for the given crate, returning the paths that were deleted.
    #[instrument(skip(self))]
    pub async fn delete_all_crate_files(&self, name: &str) -> Result<Vec<Path>> {
        let prefix = format!("{PREFIX_CRATES}/{name}").into();
        self.delete_all_with_prefix(&prefix).await
    }

    /// Deletes all READMEs for the given crate, returning the paths that were deleted.
    #[instrument(skip(self))]
    pub async fn delete_all_readmes(&self, name: &str) -> Result<Vec<Path>> {
        let prefix = format!("{PREFIX_READMES}/{name}").into();
        self.delete_all_with_prefix(&prefix).await
    }

    #[instrument(skip(self))]
    pub async fn delete(&self, key: &StorageKey<'_>) -> Result<()> {
        let path = key.path();
        self.store.delete(&path).await
    }

    /// Uploads `payload` to the location identified by `key`, storing it with
    /// the key's intended attributes.
    #[instrument(skip(self, payload))]
    pub async fn upload(&self, key: &StorageKey<'_>, payload: PutPayload) -> Result<()> {
        let attributes = self
            .supports_attributes
            .then(|| key.attributes())
            .unwrap_or_default();

        let opts = attributes.into();
        self.store.put_opts(&key.path(), payload, opts).await?;
        Ok(())
    }

    /// Uploads the contents of `reader` to the location identified by `key`,
    /// streaming it so the whole file is never buffered in memory.
    #[instrument(skip(self, reader))]
    pub async fn upload_stream(
        &self,
        key: &StorageKey<'_>,
        mut reader: impl AsyncRead + Unpin,
    ) -> anyhow::Result<()> {
        let attributes = self
            .supports_attributes
            .then(|| key.attributes())
            .unwrap_or_default();

        // Set up a streaming upload
        let mut writer = object_store::buffered::BufWriter::new(self.store.clone(), key.path())
            .with_attributes(attributes);

        // Upload the archive contents
        if let Err(error) = tokio::io::copy(&mut reader, &mut writer).await {
            // Abort the upload if something failed
            writer.abort().await?;
            return Err(error.into());
        }

        // ... or finalize upload
        writer.shutdown().await?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn download_stream(
        &self,
        key: &StorageKey<'_>,
    ) -> Result<BoxStream<'_, Result<Bytes>>> {
        let result = self.store.get(&key.path()).await?;
        Ok(result.into_stream())
    }

    #[instrument(skip(self, content))]
    pub async fn sync_index(&self, name: &str, content: Option<String>) -> Result<()> {
        let path = crates_io_index::Repository::relative_index_file_for_url(name).into();
        if let Some(content) = content {
            let attributes = self.attrs([
                (Attribute::ContentType, "text/plain"),
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

    /// This should only be used for assertions in the test suite!
    pub fn as_inner(&self) -> Arc<dyn ObjectStore> {
        self.store.clone()
    }

    async fn delete_all_with_prefix(&self, prefix: &Path) -> Result<Vec<Path>> {
        let objects = self.store.list(Some(prefix));
        let locations = objects.map(|meta| meta.map(|m| m.location)).boxed();

        let paths = self
            .store
            .delete_stream(locations)
            .try_collect::<Vec<_>>()
            .await?;

        Ok(paths)
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

#[derive(Debug)]
pub enum StorageKey<'a> {
    CrateFile { name: &'a str, version: &'a str },
    CrateZip { name: &'a str, version: &'a str },
    CrateZipManifest { name: &'a str, version: &'a str },
    Readme { name: &'a str, version: &'a str },
    OgImage { name: &'a str },
    CrateFeed { name: &'a str },
    CratesFeed,
    UpdatesFeed,
    DbDumpTar,
    DbDumpZip,
}

impl<'a> StorageKey<'a> {
    /// Builds a [`StorageKey::CrateFile`] key for the given crate version.
    pub fn for_crate_file(name: &'a str, version: &'a str) -> Self {
        StorageKey::CrateFile { name, version }
    }

    /// Builds a [`StorageKey::CrateZip`] key for the given crate version.
    pub fn for_crate_zip(name: &'a str, version: &'a str) -> Self {
        StorageKey::CrateZip { name, version }
    }

    /// Builds a [`StorageKey::CrateZipManifest`] key for the given crate version.
    pub fn for_crate_zip_manifest(name: &'a str, version: &'a str) -> Self {
        StorageKey::CrateZipManifest { name, version }
    }

    /// Builds a [`StorageKey::Readme`] key for the given crate version.
    pub fn for_readme(name: &'a str, version: &'a str) -> Self {
        StorageKey::Readme { name, version }
    }

    /// Builds a [`StorageKey::OgImage`] key for the given crate.
    pub fn for_og_image(name: &'a str) -> Self {
        StorageKey::OgImage { name }
    }

    /// Object-store path used for put/get/delete operations.
    pub fn path(&self) -> Path {
        match self {
            StorageKey::CrateFile { name, version } => {
                format!("{PREFIX_CRATES}/{name}/{name}-{version}.crate").into()
            }
            StorageKey::CrateZip { name, version } => {
                format!("{PREFIX_CRATES}/{name}/{name}-{version}.zip").into()
            }
            StorageKey::CrateZipManifest { name, version } => {
                format!("{PREFIX_CRATES}/{name}/{name}-{version}.zip.json").into()
            }
            StorageKey::Readme { name, version } => {
                format!("{PREFIX_READMES}/{name}/{name}-{version}.html").into()
            }
            StorageKey::OgImage { name } => format!("{PREFIX_OG_IMAGES}/{name}.png").into(),
            StorageKey::CrateFeed { name } => format!("rss/crates/{name}.xml").into(),
            StorageKey::CratesFeed => "rss/crates.xml".into(),
            StorageKey::UpdatesFeed => "rss/updates.xml".into(),
            StorageKey::DbDumpTar => "db-dump.tar.gz".into(),
            StorageKey::DbDumpZip => "db-dump.zip".into(),
        }
    }

    /// [`Self::path()`] rendered for a public URL, with `+` percent-encoded as
    /// `%2B`.
    pub fn cdn_path(&self) -> String {
        self.path().as_ref().replace('+', "%2B")
    }

    /// The content-type the file should be stored with, or `None` to rely on
    /// the store's default.
    pub fn content_type(&self) -> Option<&'static str> {
        match self {
            StorageKey::CrateFile { .. } => Some("application/gzip"),
            StorageKey::CrateZip { .. } => Some("application/zip"),
            StorageKey::CrateZipManifest { .. } => Some("application/json"),
            StorageKey::Readme { .. } => Some("text/html"),
            StorageKey::OgImage { .. } => Some("image/png"),
            StorageKey::CrateFeed { .. } | StorageKey::CratesFeed | StorageKey::UpdatesFeed => {
                Some("text/xml; charset=UTF-8")
            }
            StorageKey::DbDumpTar | StorageKey::DbDumpZip => None,
        }
    }

    /// The cache-control the file should be stored with, or `None` for no
    /// override.
    pub fn cache_control(&self) -> Option<&'static str> {
        match self {
            StorageKey::CrateFile { .. }
            | StorageKey::CrateZip { .. }
            | StorageKey::CrateZipManifest { .. } => Some(CACHE_CONTROL_IMMUTABLE),
            StorageKey::Readme { .. } => Some(CACHE_CONTROL_README),
            StorageKey::OgImage { .. } => Some(CACHE_CONTROL_OG_IMAGE),
            StorageKey::CrateFeed { .. }
            | StorageKey::CratesFeed
            | StorageKey::UpdatesFeed
            | StorageKey::DbDumpTar
            | StorageKey::DbDumpZip => None,
        }
    }

    /// The intended attribute set (content-type + cache-control) for the file.
    pub fn attributes(&self) -> Attributes {
        let mut attributes = Attributes::new();
        if let Some(content_type) = self.content_type() {
            attributes.insert(Attribute::ContentType, content_type.into());
        }
        if let Some(cache_control) = self.cache_control() {
            attributes.insert(Attribute::CacheControl, cache_control.into());
        }
        attributes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::body::Bytes;

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
            (
                "foo",
                "1.2.3",
                "https://static.crates.io/crates/foo/foo-1.2.3.crate",
            ),
            (
                "some-long-crate-name",
                "42.0.5-beta.1+foo",
                "https://static.crates.io/crates/some-long-crate-name/some-long-crate-name-42.0.5-beta.1%2Bfoo.crate",
            ),
        ];
        for (name, version, expected) in crate_tests {
            let key = StorageKey::for_crate_file(name, version);
            assert_eq!(storage.location(&key), expected);
        }

        let crate_zip_tests = vec![
            (
                "foo",
                "1.2.3",
                "https://static.crates.io/crates/foo/foo-1.2.3.zip",
            ),
            (
                "some-long-crate-name",
                "42.0.5-beta.1+foo",
                "https://static.crates.io/crates/some-long-crate-name/some-long-crate-name-42.0.5-beta.1%2Bfoo.zip",
            ),
        ];
        for (name, version, expected) in crate_zip_tests {
            let key = StorageKey::for_crate_zip(name, version);
            assert_eq!(storage.location(&key), expected);
        }

        let crate_zip_manifest_tests = vec![
            (
                "foo",
                "1.2.3",
                "https://static.crates.io/crates/foo/foo-1.2.3.zip.json",
            ),
            (
                "some-long-crate-name",
                "42.0.5-beta.1+foo",
                "https://static.crates.io/crates/some-long-crate-name/some-long-crate-name-42.0.5-beta.1%2Bfoo.zip.json",
            ),
        ];
        for (name, version, expected) in crate_zip_manifest_tests {
            let key = StorageKey::for_crate_zip_manifest(name, version);
            assert_eq!(storage.location(&key), expected);
        }

        let readme_tests = vec![
            (
                "foo",
                "1.2.3",
                "https://static.crates.io/readmes/foo/foo-1.2.3.html",
            ),
            (
                "some-long-crate-name",
                "42.0.5-beta.1+foo",
                "https://static.crates.io/readmes/some-long-crate-name/some-long-crate-name-42.0.5-beta.1%2Bfoo.html",
            ),
        ];
        for (name, version, expected) in readme_tests {
            let key = StorageKey::for_readme(name, version);
            assert_eq!(storage.location(&key), expected);
        }

        let og_image_tests = vec![
            ("foo", "https://static.crates.io/og-images/foo.png"),
            (
                "some-long-crate-name",
                "https://static.crates.io/og-images/some-long-crate-name.png",
            ),
        ];
        for (name, expected) in og_image_tests {
            let key = StorageKey::for_og_image(name);
            assert_eq!(storage.location(&key), expected);
        }
    }

    #[test]
    fn cdn_prefix() {
        fn storage(cdn_prefix: Option<&str>) -> Storage {
            let mut config = StorageConfig::in_memory();
            config.cdn_prefix = cdn_prefix.map(str::to_string);
            Storage::from_config(&config)
        }

        let key = StorageKey::for_og_image("foo");

        assert_eq!(storage(None).location(&key), "/og-images/foo.png");
        assert_eq!(
            storage(Some("https://fastly-static.crates.io")).location(&key),
            "https://fastly-static.crates.io/og-images/foo.png"
        );
        assert_eq!(
            storage(Some("static.crates.io")).location(&key),
            "https://static.crates.io/og-images/foo.png"
        );
        assert_eq!(
            storage(Some("static.crates.io/")).location(&key),
            "https://static.crates.io//og-images/foo.png"
        );
    }

    #[tokio::test]
    async fn delete_all_crate_files() {
        let storage = prepare().await;

        for path in ["crates/foo/foo-1.2.3.zip", "crates/foo/foo-1.2.3.zip.json"] {
            let payload = Bytes::new().into();
            storage.store.put(&path.into(), payload).await.unwrap();
        }

        let deleted_files = storage.delete_all_crate_files("foo").await.unwrap();
        assert_eq!(
            deleted_files,
            vec![
                "crates/foo/foo-1.0.0.crate".into(),
                "crates/foo/foo-1.2.3.crate".into(),
                "crates/foo/foo-1.2.3.zip".into(),
                "crates/foo/foo-1.2.3.zip.json".into(),
            ]
        );

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

        let deleted_files = storage.delete_all_readmes("foo").await.unwrap();
        assert_eq!(
            deleted_files,
            vec![
                "readmes/foo/foo-1.0.0.html".into(),
                "readmes/foo/foo-1.2.3.html".into(),
            ]
        );

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

        let key = StorageKey::for_crate_file("foo", "1.2.3");
        storage.delete(&key).await.unwrap();

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
    async fn delete_crate_zip() {
        let storage = Storage::from_config(&StorageConfig::in_memory());

        for path in ["crates/foo/foo-1.2.3.zip", "crates/foo/foo-1.2.3.zip.json"] {
            let payload = Bytes::new().into();
            storage.store.put(&path.into(), payload).await.unwrap();
        }

        let zip_key = StorageKey::for_crate_zip("foo", "1.2.3");
        storage.delete(&zip_key).await.unwrap();
        assert_eq!(
            stored_files(&storage.store).await,
            vec!["crates/foo/foo-1.2.3.zip.json"]
        );

        let manifest_key = StorageKey::for_crate_zip_manifest("foo", "1.2.3");
        storage.delete(&manifest_key).await.unwrap();
        assert!(stored_files(&storage.store).await.is_empty());
    }

    #[tokio::test]
    async fn delete_readme() {
        let storage = prepare().await;

        let key = StorageKey::for_readme("foo", "1.2.3");
        storage.delete(&key).await.unwrap();

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

        let key = StorageKey::for_crate_file("foo", "1.2.3");
        s.upload(&key, Bytes::new().into()).await.unwrap();

        let expected_files = vec!["crates/foo/foo-1.2.3.crate"];
        assert_eq!(stored_files(&s.store).await, expected_files);

        let key = StorageKey::for_crate_file("foo", "2.0.0+foo");
        s.upload(&key, Bytes::new().into()).await.unwrap();

        let expected_files = vec![
            "crates/foo/foo-1.2.3.crate",
            "crates/foo/foo-2.0.0+foo.crate",
        ];
        assert_eq!(stored_files(&s.store).await, expected_files);
    }

    #[tokio::test]
    async fn upload_crate_zip() {
        let s = Storage::from_config(&StorageConfig::in_memory());

        let key = StorageKey::for_crate_zip("foo", "1.2.3");
        s.upload_stream(&key, &b"fake zip data"[..]).await.unwrap();

        let expected_files = vec!["crates/foo/foo-1.2.3.zip"];
        assert_eq!(stored_files(&s.store).await, expected_files);

        let key = StorageKey::for_crate_zip("foo", "2.0.0+foo");
        s.upload_stream(&key, &b"fake zip data"[..]).await.unwrap();

        let expected_files = vec!["crates/foo/foo-1.2.3.zip", "crates/foo/foo-2.0.0+foo.zip"];
        assert_eq!(stored_files(&s.store).await, expected_files);
    }

    #[tokio::test]
    async fn upload_crate_zip_manifest() {
        let s = Storage::from_config(&StorageConfig::in_memory());

        let bytes = Bytes::from_static(b"{\"files\":[]}");
        let key = StorageKey::for_crate_zip_manifest("foo", "1.2.3");
        s.upload(&key, bytes.clone().into()).await.unwrap();

        let expected_files = vec!["crates/foo/foo-1.2.3.zip.json"];
        assert_eq!(stored_files(&s.store).await, expected_files);

        let key = StorageKey::for_crate_zip_manifest("foo", "2.0.0+foo");
        s.upload(&key, bytes.into()).await.unwrap();

        let expected_files = vec![
            "crates/foo/foo-1.2.3.zip.json",
            "crates/foo/foo-2.0.0+foo.zip.json",
        ];
        assert_eq!(stored_files(&s.store).await, expected_files);
    }

    #[tokio::test]
    async fn upload_readme() {
        let s = Storage::from_config(&StorageConfig::in_memory());

        let bytes = Bytes::from_static(b"hello world");
        let key = StorageKey::for_readme("foo", "1.2.3");
        s.upload(&key, bytes.clone().into()).await.unwrap();

        let expected_files = vec!["readmes/foo/foo-1.2.3.html"];
        assert_eq!(stored_files(&s.store).await, expected_files);

        let key = StorageKey::for_readme("foo", "2.0.0+foo");
        s.upload(&key, bytes.into()).await.unwrap();

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

        let key = StorageKey::DbDumpTar;
        s.upload_stream(&key, &b"fake db dump"[..]).await.unwrap();

        let expected_files = vec!["db-dump.tar.gz"];
        assert_eq!(stored_files(&s.store).await, expected_files);
    }

    #[tokio::test]
    async fn upload_og_image() {
        let s = Storage::from_config(&StorageConfig::in_memory());

        let bytes = Bytes::from_static(b"fake png data");
        let key = StorageKey::for_og_image("foo");
        s.upload(&key, bytes.clone().into()).await.unwrap();

        let expected_files = vec!["og-images/foo.png"];
        assert_eq!(stored_files(&s.store).await, expected_files);

        let key = StorageKey::for_og_image("some-long-crate-name");
        s.upload(&key, bytes.into()).await.unwrap();

        let expected_files = vec!["og-images/foo.png", "og-images/some-long-crate-name.png"];
        assert_eq!(stored_files(&s.store).await, expected_files);
    }

    #[tokio::test]
    async fn delete_og_image() {
        let s = Storage::from_config(&StorageConfig::in_memory());

        let bytes = Bytes::from_static(b"fake png data");

        let foo_key = StorageKey::for_og_image("foo");
        s.upload(&foo_key, bytes.clone().into()).await.unwrap();
        let bar_key = StorageKey::for_og_image("bar");
        s.upload(&bar_key, bytes.into()).await.unwrap();

        let expected_files = vec!["og-images/bar.png", "og-images/foo.png"];
        assert_eq!(stored_files(&s.store).await, expected_files);

        s.delete(&foo_key).await.unwrap();

        let expected_files = vec!["og-images/bar.png"];
        assert_eq!(stored_files(&s.store).await, expected_files);
    }
}
