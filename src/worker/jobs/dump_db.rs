use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use crates_io_database_dump::{create_archives, DumpDirectory};
use crates_io_worker::BackgroundJob;
use secrecy::ExposeSecret;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
pub struct DumpDb;

impl BackgroundJob for DumpDb {
    const JOB_NAME: &'static str = "dump_db";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    /// Create CSV dumps of the public information in the database, wrap them in a
    /// tarball and upload to S3.
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        const TAR_PATH: &str = "db-dump.tar.gz";
        const ZIP_PATH: &str = "db-dump.zip";

        let db_config = &env.config.db;
        let db_pool_config = db_config.replica.as_ref().unwrap_or(&db_config.primary);
        let database_url = db_pool_config.url.clone();

        let archives = spawn_blocking(move || {
            let directory = DumpDirectory::create()?;

            info!("Exporting database…");
            directory.populate(database_url.expose_secret())?;

            let export_dir = directory.path();
            info!(path = ?export_dir, "Creating tarball…");
            let tarball_prefix =
                PathBuf::from(directory.timestamp.format("%Y-%m-%d-%H%M%S").to_string());
            create_archives(export_dir, &tarball_prefix)
        })
        .await?;

        info!("Uploading tarball…");
        env.storage
            .upload_db_dump(TAR_PATH, archives.tar.path())
            .await?;
        info!("Database dump tarball uploaded");

        info!("Invalidating CDN caches…");
        if let Err(error) = env.invalidate_cdns(TAR_PATH).await {
            warn!("Failed to invalidate CDN caches: {error}");
        }

        info!("Uploading zip file…");
        env.storage
            .upload_db_dump(ZIP_PATH, archives.zip.path())
            .await?;
        info!("Database dump zip file uploaded");

        info!("Invalidating CDN caches…");
        if let Err(error) = env.invalidate_cdns(ZIP_PATH).await {
            warn!("Failed to invalidate CDN caches: {error}");
        }

        Ok(())
    }
}
