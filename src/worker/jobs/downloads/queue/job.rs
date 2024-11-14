use crate::config::CdnLogQueueConfig;
use crate::sqs::{MockSqsQueue, SqsQueue, SqsQueueImpl};
use crate::worker::jobs::ProcessCdnLog;
use crate::worker::Environment;
use anyhow::Context;
use aws_credential_types::Credentials;
use aws_sdk_sqs::config::Region;
use aws_sdk_sqs::types::Message;
use crates_io_worker::BackgroundJob;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::AsyncPgConnection;
use std::sync::Arc;

/// A background job that processes messages from the CDN log queue.
///
/// Whenever a CDN uploads a new log file to S3, AWS automatically sends a
/// message to an SQS queue. This job processes those messages, extracting the
/// log file path from each message and enqueuing a `ProcessCdnLog` job for each
/// path.
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
        info!("Processing messages from the CDN log queue…");

        let queue = build_queue(&ctx.config.cdn_log_queue);
        run(&queue, self.max_messages, &ctx.deadpool).await
    }
}

/// Builds an [SqsQueue] implementation based on the [CdnLogQueueConfig].
fn build_queue(config: &CdnLogQueueConfig) -> Box<dyn SqsQueue + Send + Sync> {
    match config {
        CdnLogQueueConfig::Mock => Box::new(MockSqsQueue::new()),
        CdnLogQueueConfig::SQS {
            access_key,
            secret_key,
            region,
            queue_url,
        } => {
            use secrecy::ExposeSecret;

            let secret_key = secret_key.expose_secret();
            let credentials = Credentials::from_keys(access_key, secret_key, None);

            let region = Region::new(region.to_owned());

            Box::new(SqsQueueImpl::new(queue_url, region, credentials))
        }
    }
}

/// Processes messages from the CDN log queue.
///
/// This function is separate from the [BackgroundJob] implementation so that it
/// can be tested without needing to construct a full [Environment] struct.
async fn run(
    queue: &impl SqsQueue,
    max_messages: usize,
    connection_pool: &Pool<AsyncPgConnection>,
) -> anyhow::Result<()> {
    const MAX_BATCH_SIZE: usize = 10;

    let mut num_remaining = max_messages;
    while num_remaining > 0 {
        let batch_size = num_remaining.min(MAX_BATCH_SIZE);
        num_remaining -= batch_size;

        debug!("Receiving next {batch_size} messages from the CDN log queue…");
        let response = queue.receive_messages(batch_size as i32).await?;

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
            process_message(message, queue, connection_pool).await?;
        }
    }

    Ok(())
}

/// Processes a single message from the CDN log queue.
#[instrument(skip_all, fields(cdn_log_queue.message.id = %message.message_id().unwrap_or("<unknown>")))]
async fn process_message(
    message: &Message,
    queue: &impl SqsQueue,
    connection_pool: &Pool<AsyncPgConnection>,
) -> anyhow::Result<()> {
    debug!("Processing message…");

    let Some(receipt_handle) = message.receipt_handle() else {
        warn!("Message has no receipt handle; skipping");
        return Ok(());
    };

    if let Some(body) = message.body() {
        process_body(body, connection_pool).await?;
        debug!("Processed message");
    } else {
        warn!("Message has no body; skipping");
    };

    debug!("Deleting message from the CDN log queue…");
    queue
        .delete_message(receipt_handle)
        .await
        .context("Failed to delete message from the CDN log queue")?;

    Ok(())
}

/// Processes a single message body from the CDN log queue.
///
/// This function only returns an `Err` if there was an error enqueueing the
/// jobs. If the message is invalid or has no records, this function logs a
/// warning and returns `Ok(())` instead. This is because we don't want to
/// requeue the message in the case of a parsing error, as it would just be
/// retried indefinitely.
async fn process_body(body: &str, connection_pool: &Pool<AsyncPgConnection>) -> anyhow::Result<()> {
    let message = match serde_json::from_str::<super::message::Message>(body) {
        Ok(message) => message,
        Err(err) => {
            warn!(%body, "Failed to parse message: {err}");
            return Ok(());
        }
    };

    if message.records.is_empty() {
        warn!("Message has no records; skipping");
        return Ok(());
    }

    let jobs = jobs_from_message(message);
    if jobs.is_empty() {
        return Ok(());
    }

    let conn = connection_pool.get().await;
    let mut conn = conn.context("Failed to acquire database connection")?;

    enqueue_jobs(jobs, &mut conn).await
}

/// Extracts a list of [`ProcessCdnLog`] jobs from a message.
fn jobs_from_message(message: super::message::Message) -> Vec<ProcessCdnLog> {
    message
        .records
        .into_iter()
        .filter_map(job_from_record)
        .collect()
}

/// Extracts a [`ProcessCdnLog`] job from a single record in a message.
///
/// If the record is for an ignored path, this function returns `None`.
///
/// If the record has an invalid path, this function logs a warning and returns
/// `None` too.
fn job_from_record(record: super::message::Record) -> Option<ProcessCdnLog> {
    let region = record.aws_region;
    let bucket = record.s3.bucket.name;
    let path = record.s3.object.key;

    if is_ignored_path(&path) {
        debug!("Skipping ignored path: {path}");
        return None;
    }

    let path = match object_store::path::Path::from_url_path(&path) {
        Ok(path) => path,
        Err(err) => {
            warn!("Failed to parse path ({path}): {err}");
            return None;
        }
    };

    Some(ProcessCdnLog::new(region, bucket, path.as_ref().to_owned()))
}

/// The CDN log files for the index domains are also stored in the same S3
/// bucket, but we know that these don't contain any crate downloads, so we
/// can ignore them.
fn is_ignored_path(path: &str) -> bool {
    path.contains("/index.staging.crates.io/") || path.contains("/index.crates.io/")
}

async fn enqueue_jobs(
    jobs: Vec<ProcessCdnLog>,
    conn: &mut AsyncPgConnection,
) -> anyhow::Result<()> {
    for job in jobs {
        let path = &job.path;

        info!("Enqueuing processing job… ({path})");
        job.async_enqueue(conn)
            .await
            .context("Failed to enqueue processing job")?;

        debug!("Enqueued processing job");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_sqs::operation::receive_message::builders::ReceiveMessageOutputBuilder;
    use aws_sdk_sqs::types::builders::MessageBuilder;
    use aws_sdk_sqs::types::Message;
    use crates_io_test_db::TestDatabase;
    use crates_io_worker::schema::background_jobs;
    use diesel::prelude::*;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    use diesel_async::RunQueryDsl;
    use insta::assert_snapshot;
    use parking_lot::Mutex;

    #[tokio::test]
    async fn test_process_cdn_log_queue() {
        crate::util::tracing::init_for_test();

        let mut queue = Box::new(MockSqsQueue::new());
        queue
            .expect_receive_messages()
            .once()
            .returning(|_max_messages| {
                Ok(ReceiveMessageOutputBuilder::default()
                    .messages(message("123", "us-west-1", "bucket", "path"))
                    .build())
            });

        queue
            .expect_receive_messages()
            .once()
            .returning(|_max_messages| Ok(ReceiveMessageOutputBuilder::default().build()));

        let deleted_handles = record_deleted_handles(&mut queue);

        let test_database = TestDatabase::new();
        let connection_pool = build_connection_pool(test_database.url());

        assert_ok!(run(&queue, 100, &connection_pool).await);

        assert_snapshot!(deleted_handles.lock().join(","), @"123");
        assert_snapshot!(open_jobs(&mut connection_pool.get().await.unwrap()).await, @"us-west-1 | bucket | path");
    }

    #[tokio::test]
    async fn test_process_cdn_log_queue_multi_page() {
        crate::util::tracing::init_for_test();

        let mut queue = Box::new(MockSqsQueue::new());
        queue
            .expect_receive_messages()
            .once()
            .returning(|_max_messages| {
                Ok(ReceiveMessageOutputBuilder::default()
                    .messages(message("1", "us-west-1", "bucket", "path1"))
                    .messages(message("2", "us-west-1", "bucket", "path2"))
                    .messages(message("3", "us-west-1", "bucket", "path3"))
                    .messages(message("4", "us-west-1", "bucket", "path4"))
                    .messages(message("5", "us-west-1", "bucket", "path5"))
                    .messages(message("6", "us-west-1", "bucket", "path6"))
                    .messages(message("7", "us-west-1", "bucket", "path7"))
                    .messages(message("8", "us-west-1", "bucket", "path8"))
                    .messages(message("9", "us-west-1", "bucket", "path9"))
                    .messages(message("10", "us-west-1", "bucket", "path10"))
                    .build())
            });

        queue
            .expect_receive_messages()
            .once()
            .returning(|_max_messages| {
                Ok(ReceiveMessageOutputBuilder::default()
                    .messages(message("11", "us-west-1", "bucket", "path11"))
                    .build())
            });

        queue
            .expect_receive_messages()
            .once()
            .returning(|_max_messages| Ok(ReceiveMessageOutputBuilder::default().build()));

        let deleted_handles = record_deleted_handles(&mut queue);

        let test_database = TestDatabase::new();
        let connection_pool = build_connection_pool(test_database.url());

        assert_ok!(run(&queue, 100, &connection_pool).await);

        assert_snapshot!(deleted_handles.lock().join(","), @"1,2,3,4,5,6,7,8,9,10,11");
        assert_snapshot!(open_jobs(&mut connection_pool.get().await.unwrap()).await, @r"
        us-west-1 | bucket | path1
        us-west-1 | bucket | path2
        us-west-1 | bucket | path3
        us-west-1 | bucket | path4
        us-west-1 | bucket | path5
        us-west-1 | bucket | path6
        us-west-1 | bucket | path7
        us-west-1 | bucket | path8
        us-west-1 | bucket | path9
        us-west-1 | bucket | path10
        us-west-1 | bucket | path11
        ");
    }

    #[tokio::test]
    async fn test_process_cdn_log_queue_parse_error() {
        crate::util::tracing::init_for_test();

        let mut queue = Box::new(MockSqsQueue::new());
        queue
            .expect_receive_messages()
            .once()
            .returning(|_max_messages| {
                let message = MessageBuilder::default()
                    .message_id("1")
                    .receipt_handle("1")
                    .body(serde_json::to_string("{}").unwrap())
                    .build();

                Ok(ReceiveMessageOutputBuilder::default()
                    .messages(message)
                    .build())
            });

        queue
            .expect_receive_messages()
            .once()
            .returning(|_max_messages| Ok(ReceiveMessageOutputBuilder::default().build()));

        let deleted_handles = record_deleted_handles(&mut queue);

        let test_database = TestDatabase::new();
        let connection_pool = build_connection_pool(test_database.url());

        assert_ok!(run(&queue, 100, &connection_pool).await);

        assert_snapshot!(deleted_handles.lock().join(","), @"1");
        assert_snapshot!(open_jobs(&mut connection_pool.get().await.unwrap()).await, @"");
    }

    #[test]
    fn test_ignored_path() {
        let valid_paths = vec![
            "cloudfront/static.crates.io/EJED5RT0WA7HA.2024-02-01-10.6a8be093.gz",
            "cloudfront/static.staging.crates.io/E6OCLKYH9FE8V.2024-02-01-10.5da9e90c.gz",
            "fastly-requests/static.crates.io/2024-02-01T09:00:00.000-4AIwSEQyIFDSzdAT1Fqt.log.zst",
            "fastly-requests/static.staging.crates.io/2024-02-01T09:00:00.000-QPF3Ea8eICqLkzaoC_Wt.log.zst"
        ];
        for path in valid_paths {
            assert!(!is_ignored_path(path));
        }

        let ignored_paths = vec![
            "cloudfront/index.crates.io/EUGCXGQIH3GQ3.2024-02-01-10.2e068fc2.gz",
            "cloudfront/index.staging.crates.io/E35K556QRQDZXW.2024-02-01-10.900ddeaf.gz",
        ];
        for path in ignored_paths {
            assert!(is_ignored_path(path));
        }
    }

    fn record_deleted_handles(queue: &mut MockSqsQueue) -> Arc<Mutex<Vec<String>>> {
        let deleted_handles = Arc::new(Mutex::new(vec![]));

        queue.expect_delete_message().returning({
            let deleted_handles = deleted_handles.clone();
            move |receipt_handle| {
                deleted_handles.lock().push(receipt_handle.to_owned());
                Ok(())
            }
        });

        deleted_handles
    }

    fn build_connection_pool(url: &str) -> Pool<AsyncPgConnection> {
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(url);
        Pool::builder(manager).build().unwrap()
    }

    fn message(id: &str, region: &str, bucket: &str, path: &str) -> Message {
        let json = json!({
            "Records": [{
                "awsRegion": region,
                "s3": {
                    "bucket": { "name": bucket },
                    "object": { "key": path },
                }
            }]
        });

        MessageBuilder::default()
            .message_id(id)
            .receipt_handle(id)
            .body(serde_json::to_string(&json).unwrap())
            .build()
    }

    async fn open_jobs(conn: &mut AsyncPgConnection) -> String {
        let jobs = background_jobs::table
            .select((background_jobs::job_type, background_jobs::data))
            .load::<(String, serde_json::Value)>(conn)
            .await
            .unwrap();

        jobs.into_iter()
            .inspect(|(job_type, _data)| assert_eq!(job_type, ProcessCdnLog::JOB_NAME))
            .map(|(_job_type, data)| data)
            .map(|data| serde_json::from_value::<ProcessCdnLog>(data).unwrap())
            .map(|job| format!("{} | {} | {}", job.region, job.bucket, job.path))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
