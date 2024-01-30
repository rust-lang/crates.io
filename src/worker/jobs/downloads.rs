mod message;

use crate::config::CdnLogStorageConfig;
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use anyhow::Context;
use aws_credential_types::Credentials;
use aws_sdk_sqs::config::{BehaviorVersion, Region};
use crates_io_cdn_logs::{count_downloads, Decompressor};
use crates_io_env_vars::required_var;
use crates_io_worker::BackgroundJob;
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::memory::InMemory;
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

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let store = self
            .build_store(&ctx.config.cdn_log_storage)
            .context("Failed to build object store")?;

        self.run(store).await
    }
}

impl ProcessCdnLog {
    fn build_store(&self, config: &CdnLogStorageConfig) -> anyhow::Result<Box<dyn ObjectStore>> {
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

                Ok(Box::new(store))
            }
            CdnLogStorageConfig::Local { path } => {
                Ok(Box::new(LocalFileSystem::new_with_prefix(path)?))
            }
            CdnLogStorageConfig::Memory => Ok(Box::new(InMemory::new())),
        }
    }

    async fn run(&self, store: Box<dyn ObjectStore>) -> anyhow::Result<()> {
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

#[derive(Debug, Serialize, Deserialize, clap::Parser)]
pub struct ProcessCdnLogQueue {
    /// The maximum number of messages to receive from the queue and process.
    #[clap(long, default_value = "1")]
    max_messages: usize,
}

impl BackgroundJob for ProcessCdnLogQueue {
    const JOB_NAME: &'static str = "process_cdn_log_queue";

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        const MAX_BATCH_SIZE: usize = 10;

        let access_key = required_var("CDN_LOG_QUEUE_ACCESS_KEY")?;
        let secret_key = required_var("CDN_LOG_QUEUE_SECRET_KEY")?;
        let queue_url = required_var("CDN_LOG_QUEUE_URL")?;
        let region = required_var("CDN_LOG_QUEUE_REGION")?;

        let credentials = Credentials::from_keys(access_key, secret_key, None);

        let config = aws_sdk_sqs::Config::builder()
            .credentials_provider(credentials)
            .region(Region::new(region))
            .behavior_version(BehaviorVersion::v2023_11_09())
            .build();

        let client = aws_sdk_sqs::Client::from_conf(config);

        info!("Receiving messages from the CDN log queue…");
        let mut num_remaining = self.max_messages;
        while num_remaining > 0 {
            let batch_size = num_remaining.min(MAX_BATCH_SIZE);
            num_remaining -= batch_size;

            debug!("Receiving next {batch_size} messages from the CDN log queue…");
            let response = client
                .receive_message()
                .queue_url(&queue_url)
                .max_number_of_messages(batch_size as i32)
                .send()
                .await?;

            let messages = response.messages();
            debug!(
                "Received {num_messages} messages from the CDN log queue",
                num_messages = messages.len()
            );
            if messages.is_empty() {
                info!("No more messages to receive from the CDN log queue");
                break;
            }

            for message in messages {
                let message_id = message.message_id().unwrap_or("<unknown>");
                debug!("Processing message: {message_id}");

                let Some(receipt_handle) = message.receipt_handle() else {
                    warn!("Message {message_id} has no receipt handle; skipping");
                    continue;
                };

                debug!("Deleting message {message_id} from the CDN log queue…");
                client
                    .delete_message()
                    .queue_url(&queue_url)
                    .receipt_handle(receipt_handle)
                    .send()
                    .await
                    .with_context(|| {
                        format!("Failed to delete message {message_id} from the CDN log queue")
                    })?;

                let Some(body) = message.body() else {
                    warn!("Message {message_id} has no body; skipping");
                    continue;
                };

                let message = match serde_json::from_str::<message::Message>(body) {
                    Ok(message) => message,
                    Err(err) => {
                        warn!("Failed to parse message {message_id}: {err}");
                        continue;
                    }
                };

                if message.records.is_empty() {
                    warn!("Message {message_id} has no records; skipping");
                    continue;
                }

                let pool = ctx.connection_pool.clone();
                spawn_blocking({
                    let message_id = message_id.to_owned();
                    move || {
                        let mut conn = pool
                            .get()
                            .context("Failed to acquire database connection")?;

                        for record in message.records {
                            let region = record.aws_region;
                            let bucket = record.s3.bucket.name;
                            let path = record.s3.object.key;

                            let path = match object_store::path::Path::from_url_path(&path) {
                                Ok(path) => path,
                                Err(err) => {
                                    warn!("Failed to parse path ({path}): {err}");
                                    continue;
                                }
                            };

                            info!("Enqueuing processing job for message {message_id}… ({path})");
                            let job = ProcessCdnLog::new(region, bucket, path.as_ref().to_owned());

                            job.enqueue(&mut conn).with_context(|| {
                                format!("Failed to enqueue processing job for message {message_id}")
                            })?;

                            debug!("Enqueued processing job for message {message_id}");
                        }

                        Ok::<_, anyhow::Error>(())
                    }
                })
                .await?;

                debug!("Processed message: {message_id}");
            }
        }

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
                include_bytes!("../../../crates_io_cdn_logs/test_data/cloudfront/basic.log.gz");

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
