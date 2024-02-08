use crate::config::CdnLogStorageConfig;
use crate::worker::Environment;
use anyhow::Context;
use crates_io_cdn_logs::{count_downloads, Decompressor};
use crates_io_worker::BackgroundJob;
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::memory::InMemory;
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
        let store = self
            .build_store(&ctx.config.cdn_log_storage)
            .context("Failed to build object store")?;

        self.run(store).await
    }
}

impl ProcessCdnLog {
    /// Builds an object store based on the [CdnLogStorageConfig] and the
    /// `region` and `bucket` fields of the [ProcessCdnLog] struct.
    ///
    /// If the passed in [CdnLogStorageConfig] is using local file or in-memory
    /// storage the `region` and `bucket` fields are ignored.
    fn build_store(&self, config: &CdnLogStorageConfig) -> anyhow::Result<Arc<dyn ObjectStore>> {
        match config {
            CdnLogStorageConfig::S3 {
                access_key,
                secret_key,
            } => {
                use secrecy::ExposeSecret;

                let store = AmazonS3Builder::new()
                    .with_region(&self.region)
                    .with_bucket_name(&self.bucket)
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

    /// Runs the background job with the given object store.
    ///
    /// This method is separate from the `BackgroundJob` trait method so that
    /// it can be tested without having to construct a full[Environment]
    /// struct.
    async fn run(&self, store: Arc<dyn ObjectStore>) -> anyhow::Result<()> {
        let path = object_store::path::Path::parse(&self.path)
            .with_context(|| format!("Failed to parse path: {:?}", self.path))?;

        let meta = store.head(&path).await;
        let meta = meta.with_context(|| format!("Failed to request metadata for {path:?}"))?;

        let reader = object_store::buffered::BufReader::new(store, &meta);
        let decompressor = Decompressor::from_extension(reader, path.extension())?;
        let reader = BufReader::new(decompressor);

        let downloads = count_downloads(reader).await?;

        // TODO: for now this background job just prints out the results, but
        // eventually it should insert them into the database instead.

        if downloads.is_empty() {
            info!("No downloads found in log file: {path}");
            return Ok(());
        }

        let num_crates = downloads.unique_crates().len();
        let total_inserts = downloads.len();
        let total_downloads = downloads.sum_downloads();

        info!("Log file: {path}");
        info!("Number of crates: {num_crates}");
        info!("Number of needed inserts: {total_inserts}");
        info!("Total number of downloads: {total_downloads}");

        let mut downloads = downloads.into_vec();
        downloads.sort_by_key(|(_, _, _, downloads)| Reverse(*downloads));

        let top_downloads = downloads
            .into_iter()
            .take(30)
            .map(|(krate, version, date, downloads)| {
                format!("{date}  {krate}@{version} .. {downloads}")
            })
            .collect::<Vec<_>>();

        info!("Top 30 downloads: {top_downloads:?}");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_cdn_log() {
        let _guard = crate::util::tracing::init_for_test();

        let path = "cloudfront/index.staging.crates.io/E35K556QRQDZXW.2024-01-16-16.d01d5f13.gz";

        let job = ProcessCdnLog::new(
            "us-west-1".to_string(),
            "bucket".to_string(),
            path.to_string(),
        );

        let config = CdnLogStorageConfig::memory();
        let store = assert_ok!(job.build_store(&config));

        // Add dummy data into the store
        {
            let bytes =
                include_bytes!("../../../../crates_io_cdn_logs/test_data/cloudfront/basic.log.gz");

            store.put(&path.into(), bytes[..].into()).await.unwrap();
        }

        assert_ok!(job.run(store).await);
    }

    #[tokio::test]
    async fn test_s3_builder() {
        let path = "cloudfront/index.staging.crates.io/E35K556QRQDZXW.2024-01-16-16.d01d5f13.gz";

        let job = ProcessCdnLog::new(
            "us-west-1".to_string(),
            "bucket".to_string(),
            path.to_string(),
        );

        let access_key = "access_key".into();
        let secret_key = "secret_key".to_string().into();
        let config = CdnLogStorageConfig::s3(access_key, secret_key);
        assert_ok!(job.build_store(&config));
    }
}
