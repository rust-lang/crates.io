use crate::config::CdnLogQueueConfig;
use crate::db::DieselPool;
use crate::sqs::{MockSqsQueue, SqsQueue, SqsQueueImpl};
use crate::tasks::spawn_blocking;
use crate::worker::jobs::ProcessCdnLog;
use crate::worker::Environment;
use anyhow::Context;
use aws_credential_types::Credentials;
use aws_sdk_sqs::config::Region;
use crates_io_worker::BackgroundJob;
use std::sync::Arc;

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
        run(queue, self.max_messages, &ctx.connection_pool).await
    }
}

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

async fn run(
    queue: Box<dyn SqsQueue + Send + Sync>,
    max_messages: usize,
    connection_pool: &DieselPool,
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
            let message_id = message.message_id().unwrap_or("<unknown>");
            debug!("Processing message: {message_id}");

            let Some(receipt_handle) = message.receipt_handle() else {
                warn!("Message {message_id} has no receipt handle; skipping");
                continue;
            };

            debug!("Deleting message {message_id} from the CDN log queue…");
            queue
                .delete_message(receipt_handle)
                .await
                .with_context(|| {
                    format!("Failed to delete message {message_id} from the CDN log queue")
                })?;

            let Some(body) = message.body() else {
                warn!("Message {message_id} has no body; skipping");
                continue;
            };

            let message = match serde_json::from_str::<super::message::Message>(body) {
                Ok(message) => message,
                Err(err) => {
                    warn!(%body, "Failed to parse message {message_id}: {err}");
                    continue;
                }
            };

            if message.records.is_empty() {
                warn!("Message {message_id} has no records; skipping");
                continue;
            }

            let jobs = message
                .records
                .into_iter()
                .filter_map(|record| {
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
                })
                .collect::<Vec<_>>();

            let pool = connection_pool.clone();
            spawn_blocking({
                let message_id = message_id.to_owned();
                move || {
                    let mut conn = pool
                        .get()
                        .context("Failed to acquire database connection")?;

                    for job in jobs {
                        let path = &job.path;
                        info!("Enqueuing processing job for message {message_id}… ({path})");
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

fn is_ignored_path(path: &str) -> bool {
    path.contains("/index.staging.crates.io/") || path.contains("/index.crates.io/")
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
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::QueryDsl;
    use insta::assert_snapshot;
    use parking_lot::Mutex;

    #[tokio::test]
    async fn test_process_cdn_log_queue() {
        let _guard = crate::util::tracing::init_for_test();

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

        assert_ok!(run(queue, 100, &connection_pool).await);

        assert_snapshot!(deleted_handles.lock().join(","), @"123");
        assert_snapshot!(open_jobs(&mut test_database.connect()), @"us-west-1 | bucket | path");
    }

    #[tokio::test]
    async fn test_process_cdn_log_queue_multi_page() {
        let _guard = crate::util::tracing::init_for_test();

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

        assert_ok!(run(queue, 100, &connection_pool).await);

        assert_snapshot!(deleted_handles.lock().join(","), @"1,2,3,4,5,6,7,8,9,10,11");
        assert_snapshot!(open_jobs(&mut test_database.connect()), @r###"
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
        "###);
    }

    #[tokio::test]
    async fn test_process_cdn_log_queue_parse_error() {
        let _guard = crate::util::tracing::init_for_test();

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

        assert_ok!(run(queue, 100, &connection_pool).await);

        assert_snapshot!(deleted_handles.lock().join(","), @"1");
        assert_snapshot!(open_jobs(&mut test_database.connect()), @"");
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

    fn build_connection_pool(url: &str) -> DieselPool {
        let pool = Pool::builder().build(ConnectionManager::new(url)).unwrap();
        DieselPool::new_background_worker(pool)
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

    fn open_jobs(conn: &mut PgConnection) -> String {
        let jobs = background_jobs::table
            .select((background_jobs::job_type, background_jobs::data))
            .load::<(String, serde_json::Value)>(conn)
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
