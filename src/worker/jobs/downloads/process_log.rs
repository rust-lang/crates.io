use crate::config::CdnLogStorageConfig;
use crate::worker::Environment;
use anyhow::Context;
use crates_io_cdn_logs::{count_downloads, Decompressor, DownloadsMap};
use crates_io_worker::BackgroundJob;
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::memory::InMemory;
use object_store::path::Path;
use object_store::ObjectStore;
use std::cmp::Reverse;
use std::sync::Arc;
use tokio::io::BufReader;

/// A background job that loads a CDN log file from an object store (aka. S3),
/// counts the number of downloads for each crate and version, and then inserts
/// the results into the database.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessCdnLog {
    pub region: String,
    pub bucket: String,
    pub path: String,
}

impl ProcessCdnLog {
    pub fn new(region: String, bucket: String, path: String) -> Self {
        Self {
            region,
            bucket,
            path,
        }
    }
}

impl BackgroundJob for ProcessCdnLog {
    const JOB_NAME: &'static str = "process_cdn_log";

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        // The store is rebuilt for each run because we don't want to assume
        // that all log files live in the same AWS region or bucket, and those
        // two pieces are necessary for the store construction.
        let store = build_store(&ctx.config.cdn_log_storage, &self.region, &self.bucket)
            .context("Failed to build object store")?;

        run(&self.path, store).await
    }
}

/// Builds an object store based on the [CdnLogStorageConfig] and the
/// `region` and `bucket` arguments.
///
/// If the passed in [CdnLogStorageConfig] is using local file or in-memory
/// storage the `region` and `bucket` arguments are ignored.
fn build_store(
    config: &CdnLogStorageConfig,
    region: impl Into<String>,
    bucket: impl Into<String>,
) -> anyhow::Result<Arc<dyn ObjectStore>> {
    match config {
        CdnLogStorageConfig::S3 {
            access_key,
            secret_key,
        } => {
            use secrecy::ExposeSecret;

            let store = AmazonS3Builder::new()
                .with_region(region.into())
                .with_bucket_name(bucket.into())
                .with_access_key_id(access_key)
                .with_secret_access_key(secret_key.expose_secret())
                .build()?;

            Ok(Arc::new(store))
        }
        CdnLogStorageConfig::Local { path } => {
            Ok(Arc::new(LocalFileSystem::new_with_prefix(path)?))
        }
        CdnLogStorageConfig::Memory => Ok(Arc::new(InMemory::new())),
    }
}

/// Loads the given log file from the object store and counts the number of
/// downloads for each crate and version. The results are printed to the log.
///
/// This function is separate from the [`BackgroundJob`] trait method so that
/// it can be tested without having to construct a full [`Environment`]
/// struct.
#[instrument(skip_all, fields(cdn_log_store.path = %path))]
async fn run(path: &str, store: Arc<dyn ObjectStore>) -> anyhow::Result<()> {
    let path = Path::parse(path).with_context(|| format!("Failed to parse path: {path:?}"))?;

    let downloads = load_and_count(&path, store).await?;

    // TODO: for now this background job just prints out the results, but
    // eventually it should insert them into the database instead.

    if downloads.is_empty() {
        info!("No downloads found in log file");
        return Ok(());
    }

    log_stats(&downloads);
    log_top_downloads(downloads, 30);

    Ok(())
}

/// Loads the given log file from the object store and counts the number of
/// downloads for each crate and version.
async fn load_and_count(path: &Path, store: Arc<dyn ObjectStore>) -> anyhow::Result<DownloadsMap> {
    let meta = store.head(path).await;
    let meta = meta.with_context(|| format!("Failed to request metadata for {path:?}"))?;

    let reader = object_store::buffered::BufReader::new(store, &meta);
    let decompressor = Decompressor::from_extension(reader, path.extension())?;
    let reader = BufReader::new(decompressor);

    count_downloads(reader).await
}

/// Prints the total number of downloads, the number of crates, and the number
/// of needed inserts to the log.
fn log_stats(downloads: &DownloadsMap) {
    let total_downloads = downloads.sum_downloads();
    info!("Total number of downloads: {total_downloads}");

    let num_crates = downloads.unique_crates().len();
    info!("Number of crates: {num_crates}");

    let total_inserts = downloads.len();
    info!("Number of needed inserts: {total_inserts}");
}

/// Prints the top `num` downloads from the given [`DownloadsMap`] map to the log.
fn log_top_downloads(downloads: DownloadsMap, num: usize) {
    let mut downloads = downloads.into_vec();
    downloads.sort_by_key(|(_, _, _, downloads)| Reverse(*downloads));

    let top_downloads = downloads
        .into_iter()
        .take(num)
        .map(|(krate, version, date, downloads)| {
            format!("{date}  {krate}@{version} .. {downloads}")
        })
        .collect::<Vec<_>>();

    info!("Top {num} downloads: {top_downloads:?}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_cdn_log() {
        let _guard = crate::util::tracing::init_for_test();

        let path = "cloudfront/static.crates.io/E35K556QRQDZXW.2024-01-16-16.d01d5f13.gz";

        let config = CdnLogStorageConfig::memory();
        let store = assert_ok!(build_store(&config, "us-west-1", "bucket"));

        // Add dummy data into the store
        {
            let bytes =
                include_bytes!("../../../../crates_io_cdn_logs/test_data/cloudfront/basic.log.gz");

            store.put(&path.into(), bytes[..].into()).await.unwrap();
        }

        assert_ok!(run(path, store).await);
    }

    #[tokio::test]
    async fn test_s3_builder() {
        let access_key = "access_key".into();
        let secret_key = "secret_key".to_string().into();
        let config = CdnLogStorageConfig::s3(access_key, secret_key);
        assert_ok!(build_store(&config, "us-west-1", "bucket"));
    }
}
