use crate::storage::StorageKey;
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use crates_io_database::models::CloudFrontDistribution;
use crates_io_database_dump::{DumpDirectory, create_archives};
use crates_io_worker::BackgroundJob;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
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
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        let db_config = &env.config.db;
        let db_pool_config = db_config.replica.as_ref().unwrap_or(&db_config.primary);
        let database_url = db_pool_config.url.clone();
        let postgres_bin_dir = env.config.postgres_bin_dir.clone();
        let schema = self.schema.clone();

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
        env.storage.upload_stream(&tar_key, tar_file).await?;
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
        env.storage.upload_stream(&zip_key, zip_file).await?;
        info!("Database dump zip file uploaded");

        info!("Invalidating CDN caches…");
        if let Err(error) = env.invalidate_cdns(&conn, dist, &zip_key.cdn_path()).await {
            warn!("Failed to invalidate CDN caches: {error}");
        }

        Ok(())
    }
}
