use crate::schema::{crates, versions};
use crate::storage::Storage;
use crate::worker::Environment;
use crate::worker::jobs::GenerateOgImage;
use anyhow::Context;
use async_compression::tokio::bufread::GzipDecoder;
use crates_io_database::schema::default_versions;
use crates_io_linecount::{LinecountStats, PathDetails};
use crates_io_worker::BackgroundJob;
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, BufReader};
use tokio_util::io::StreamReader;
use tracing::{info, instrument, warn};

#[derive(Clone, Serialize, Deserialize)]
pub struct AnalyzeCrateFile {
    version_id: i32,
}

impl AnalyzeCrateFile {
    pub fn new(version_id: i32) -> Self {
        Self { version_id }
    }
}

impl BackgroundJob for AnalyzeCrateFile {
    const JOB_NAME: &'static str = "analyze_crate_file";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    #[instrument(skip_all, fields(version_id = ?self.version_id))]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        let version_id = self.version_id;

        info!("Starting crate file analysis… (version_id={version_id})");

        let start = Instant::now();

        let mut conn = env.deadpool.get().await?;

        let Some((krate, version)) = get_crate_version_info(version_id, &mut conn).await? else {
            warn!("version_id={version_id} not found in database, skipping analysis");
            return Ok(());
        };

        info!("Loading and analyzing crate file for {krate}@{version}… (version_id={version_id})");
        let linecount_stats = analyze_crate_tarball(&krate, &version, &env.storage).await?;

        update_version_linecount_stats(version_id, &linecount_stats, &mut conn).await?;

        info!(
            duration = start.elapsed().as_nanos(),
            "Crate file analysis completed for {krate}@{version} (version_id={version_id})"
        );

        if let Err(err) = handle_og_image_rerender(&krate, version_id, &mut conn).await {
            warn!(
                "Failed to schedule OG image rerender for {krate}@{version} (version_id={version_id}): {err}"
            );
        }

        Ok(())
    }
}

/// Retrieves crate name and version number for a given version ID
#[instrument(skip(conn))]
async fn get_crate_version_info(
    version_id: i32,
    conn: &mut AsyncPgConnection,
) -> anyhow::Result<Option<(String, String)>> {
    versions::table
        .find(version_id)
        .inner_join(crates::table)
        .select((crates::name, versions::num))
        .first::<(String, String)>(conn)
        .await
        .optional()
        .context("Failed to query crate and version info")
}

/// Downloads and analyzes a crate tarball to generate linecount statistics
#[instrument(skip(storage))]
async fn analyze_crate_tarball(
    krate: &str,
    version: &str,
    storage: &Storage,
) -> anyhow::Result<LinecountStats> {
    let result = storage.download_crate_file(krate, version).await;
    let stream = result.context("Failed to download crate file")?;
    let reader = StreamReader::new(stream);
    let reader = BufReader::new(reader);
    let decoder = GzipDecoder::new(reader);

    let mut archive = tokio_tar::Archive::new(decoder);

    let entries = archive.entries();
    let mut entries = entries.context("Failed to read tarball entries")?;

    let mut linecount_stats = LinecountStats::new();
    while let Some(entry) = entries.next().await {
        let mut entry = entry.context("Failed to read tarball entry")?;
        if !entry.header().entry_type().is_file() {
            // Skip directories and other non-file entries
            continue;
        }

        let path = entry.path().context("Failed to get entry path")?;
        let path_details = PathDetails::from_path(&path);

        // Check if this file should be counted for line statistics
        if !path_details.should_ignore()
            && let Some(language_type) = path_details.language_type()
        {
            // If this is a file that we want to count, read it and update the linecount stats.
            let mut contents = Vec::new();
            let result = entry.read_to_end(&mut contents).await;
            result.context("Failed to read entry contents")?;

            linecount_stats.add_file(language_type, &contents);
        }
    }

    Ok(linecount_stats)
}

/// Updates the linecount statistics for a version in the database
#[instrument(skip(conn, linecount_stats))]
async fn update_version_linecount_stats(
    version_id: i32,
    linecount_stats: &LinecountStats,
    conn: &mut AsyncPgConnection,
) -> anyhow::Result<()> {
    let linecount_stats =
        serde_json::to_value(linecount_stats).context("Failed to serialize linecount stats")?;

    diesel::update(versions::table.find(version_id))
        .set(versions::linecounts.eq(linecount_stats))
        .execute(conn)
        .await
        .context("Failed to save linecount stats to the database")?;

    Ok(())
}

/// Check whether the `version_id` is a default version of any crate and
/// schedule an OpenGraph image rerender background job if that is the case.
#[instrument(skip(conn))]
async fn handle_og_image_rerender(
    crate_name: &str,
    version_id: i32,
    conn: &mut AsyncPgConnection,
) -> anyhow::Result<()> {
    let is_default_version = diesel::select(exists(
        default_versions::table.filter(default_versions::version_id.eq(version_id)),
    ))
    .get_result::<bool>(conn)
    .await?;

    if is_default_version {
        GenerateOgImage::new(crate_name.to_string())
            .enqueue(conn)
            .await?;
    }

    Ok(())
}
