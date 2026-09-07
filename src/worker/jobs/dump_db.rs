use crate::datadog::common_tags;
use crate::storage::StorageKey;
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use chrono::{DateTime, Utc};
use crates_io_database::models::CloudFrontDistribution;
use crates_io_database_dump::{DumpDirectory, create_archives};
use crates_io_datadog::{DatadogClient, MetricType, Point, Resource, Series};
use crates_io_worker::BackgroundJob;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DumpDb {
    /// Optional Postgres schema to restrict the dump to. `None` (the
    /// production default) dumps every schema in the database. The test
    /// harness sets this to the per-test schema so `pg_dump` doesn't race
    /// with concurrent test schemas.
    #[serde(default)]
    schema: Option<String>,
}

impl DumpDb {
    /// Convenience constructor that scopes the dump to a single Postgres
    /// schema.
    pub fn for_schema(schema: impl Into<String>) -> Self {
        Self {
            schema: Some(schema.into()),
        }
    }
}

impl BackgroundJob for DumpDb {
    const JOB_NAME: &'static str = "dump_db";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    /// Creates CSV dumps of the public information in the database, wraps them in a
    /// tarball and uploads to S3.
    async fn run(self, env: Self::Context) -> anyhow::Result<()> {
        let db_config = &env.config.db;
        let db_pool_config = db_config.replica.as_ref().unwrap_or(&db_config.primary);
        let database_url = db_pool_config.url.clone();
        let postgres_bin_dir = env.config.postgres_bin_dir.clone();
        let schema = self.schema;

        let archives = spawn_blocking(move || {
            let directory = DumpDirectory::create(postgres_bin_dir)?;

            info!("Exporting database…");
            directory.populate(database_url.expose_secret(), schema.as_deref())?;

            let export_dir = directory.path();
            info!(path = ?export_dir, "Creating tarball…");
            let tarball_prefix = PathBuf::from(directory.timestamp.format("%F-%H%M%S").to_string());
            create_archives(export_dir, &tarball_prefix)
        })
        .await??;

        info!("Uploading tarball…");
        let tar_key = StorageKey::DbDumpTar;
        let tar_file = tokio::fs::File::open(archives.tar.path()).await?;
        let size = tar_file.metadata().await?.len();
        let upload_start = Instant::now();
        env.storage.upload_stream(&tar_key, tar_file).await?;
        let upload_duration = upload_start.elapsed();
        if let Some(datadog) = &env.datadog {
            let domain = &env.config.domain_name;
            let result =
                report_dump_metrics(datadog, domain, "tar.gz", size, upload_duration).await;
            if let Err(error) = result {
                warn!("Failed to submit database dump metrics: {error:#}");
            }
        }
        info!("Database dump tarball uploaded");

        info!("Invalidating CDN caches…");
        let conn = env.deadpool.get().await?;
        let dist = CloudFrontDistribution::Static;

        if let Err(error) = env.invalidate_cdns(&conn, dist, &tar_key.cdn_path()).await {
            warn!("Failed to invalidate CDN caches: {error}");
        }

        info!("Uploading zip file…");
        let zip_key = StorageKey::DbDumpZip;
        let zip_file = tokio::fs::File::open(archives.zip.path()).await?;
        let size = zip_file.metadata().await?.len();
        let upload_start = Instant::now();
        env.storage.upload_stream(&zip_key, zip_file).await?;
        let upload_duration = upload_start.elapsed();
        if let Some(datadog) = &env.datadog {
            let domain = &env.config.domain_name;
            let result = report_dump_metrics(datadog, domain, "zip", size, upload_duration).await;
            if let Err(error) = result {
                warn!("Failed to submit database dump metrics: {error:#}");
            }
        }
        info!("Database dump zip file uploaded");

        info!("Invalidating CDN caches…");
        if let Err(error) = env.invalidate_cdns(&conn, dist, &zip_key.cdn_path()).await {
            warn!("Failed to invalidate CDN caches: {error}");
        }

        Ok(())
    }
}

/// Submits the compressed archive size in bytes and successful upload time in nanoseconds.
async fn report_dump_metrics(
    datadog: &DatadogClient,
    domain: &str,
    format: &str,
    size: u64,
    upload_duration: Duration,
) -> anyhow::Result<()> {
    let timestamp = Utc::now();
    let series = dump_metric_series(domain, format, size, upload_duration, timestamp);
    datadog.submit_metrics(&series).await
}

/// Builds archive size and upload duration metrics at the supplied time.
fn dump_metric_series(
    domain: &str,
    format: &str,
    size: u64,
    upload_duration: Duration,
    timestamp: DateTime<Utc>,
) -> [Series; 2] {
    let timestamp = timestamp.timestamp();
    let mut tags = common_tags(domain);
    tags.push(format!("format:{format}"));
    let host = Resource::builder().kind("host").name(domain).build();
    let upload_ns = upload_duration.as_nanos() as f64;
    [
        ("crates_io.db_dump_size_bytes", size as f64),
        ("crates_io.db_dump_upload_duration_ns", upload_ns),
    ]
    .map(|(metric, value)| {
        let point = Point::builder().timestamp(timestamp).value(value).build();
        Series::builder()
            .metric(metric)
            .kind(MetricType::Gauge)
            .points(vec![point])
            .resources(vec![host.clone()])
            .tags(tags.clone())
            .build()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks the archive measurements, units, timestamp, and identifying tags.
    #[test]
    fn builds_dump_metric_series() {
        let duration = Duration::from_millis(1250);
        let timestamp = DateTime::from_timestamp(1000, 0).unwrap();
        let series = dump_metric_series("staging.crates.io", "tar.gz", 4096, duration, timestamp);
        insta::assert_json_snapshot!(series);
    }
}
