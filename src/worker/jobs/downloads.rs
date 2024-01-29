use crate::worker::Environment;
use anyhow::Context;
use crates_io_cdn_logs::{count_downloads, Decompressor};
use crates_io_env_vars::required_var;
use crates_io_worker::BackgroundJob;
use object_store::aws::AmazonS3Builder;
use object_store::ObjectStore;
use std::cmp::Reverse;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::BufReader;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessCdnLog {
    region: String,
    bucket: String,
    path: String,
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

    async fn run(&self, _ctx: Self::Context) -> anyhow::Result<()> {
        let access_key = required_var("AWS_ACCESS_KEY")?;
        let secret_key = required_var("AWS_SECRET_KEY")?;

        let store = AmazonS3Builder::new()
            .with_region(&self.region)
            .with_bucket_name(&self.bucket)
            .with_access_key_id(access_key)
            .with_secret_access_key(secret_key)
            .build()
            .context("Failed to build object store")?;

        let path = object_store::path::Path::parse(&self.path)
            .with_context(|| format!("Failed to parse path: {:?}", self.path))?;

        let meta = store.head(&path).await;
        let meta = meta.with_context(|| format!("Failed to request metadata for {path:?}"))?;

        let reader = object_store::buffered::BufReader::new(Arc::new(store), &meta);
        let decompressor = Decompressor::from_extension(reader, path.extension())?;
        let reader = BufReader::new(decompressor);

        let parse_start = Instant::now();
        let downloads = count_downloads(reader).await?;
        let parse_duration = parse_start.elapsed();

        // TODO: for now this background job just prints out the results, but
        // eventually it should insert them into the database instead.

        if downloads.as_inner().is_empty() {
            info!("No downloads found in log file: {path}");
            return Ok(());
        }

        let num_crates = downloads
            .as_inner()
            .iter()
            .map(|((_, krate, _), _)| krate)
            .collect::<HashSet<_>>()
            .len();

        let total_inserts = downloads.as_inner().len();

        let total_downloads = downloads
            .as_inner()
            .iter()
            .map(|(_, downloads)| downloads)
            .sum::<u64>();

        info!("Log file: {path}");
        info!("Number of crates: {num_crates}");
        info!("Number of needed inserts: {total_inserts}");
        info!("Total number of downloads: {total_downloads}");
        info!("Time to parse: {parse_duration:?}");

        let mut downloads = downloads.into_inner().into_iter().collect::<Vec<_>>();
        downloads.sort_by_key(|((_, _, _), downloads)| Reverse(*downloads));

        let top_downloads = downloads
            .into_iter()
            .take(30)
            .map(|((krate, version, date), downloads)| {
                format!("{date}  {krate}@{version} .. {downloads}")
            })
            .collect::<Vec<_>>();

        info!("Top 30 downloads: {top_downloads:?}");

        Ok(())
    }
}
