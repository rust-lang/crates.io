use crate::schema::{crates, versions};
use crate::storage::StorageKey;
use crate::worker::Environment;
use anyhow::Context;
use chrono::{DateTime, Datelike, Timelike, Utc};
use crates_io_crate_zip::build_zip;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::task::spawn_blocking;
use tracing::{info, instrument, warn};

/// Builds the seekable `.zip` source archive and its `.zip.json` manifest for a
/// single version, uploads both, and records their checksums on the version.
#[derive(Clone, Serialize, Deserialize)]
pub struct BuildCrateZip {
    version_id: i32,
}

impl BuildCrateZip {
    pub fn new(version_id: i32) -> Self {
        Self { version_id }
    }
}

impl BackgroundJob for BuildCrateZip {
    const JOB_NAME: &'static str = "build_crate_zip";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    #[instrument(skip_all, fields(version_id = ?self.version_id))]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        let version_id = self.version_id;

        info!("Starting zip build… (version_id={version_id})");

        let start = Instant::now();

        let mut conn = env.deadpool.get().await?;

        let Some(info) = CrateVersionInfo::load(version_id, &conn).await? else {
            warn!("version_id={version_id} not found in database, skipping zip build");
            return Ok(());
        };

        let name = info.name.as_str();
        let version = info.version.as_str();
        let created_at = info.created_at()?;

        info!("Building zip for {name}@{version}… (version_id={version_id})");

        let tarball = download_to_tempfile(&env.storage, name, version).await?;

        let artifacts = spawn_blocking(move || Artifacts::build(tarball, created_at))
            .await
            .context("Zip build task panicked")??;

        let zip_key = StorageKey::for_crate_zip(name, version);
        env.storage
            .upload_crate_zip(&zip_key, tokio::fs::File::from_std(artifacts.zip))
            .await
            .context("Failed to upload zip archive")?;

        let manifest_key = StorageKey::for_crate_zip_manifest(name, version);
        env.storage
            .upload(&manifest_key, artifacts.manifest_json.into())
            .await
            .context("Failed to upload zip manifest")?;

        diesel::update(versions::table.find(version_id))
            .set((
                versions::zip_sha256.eq(artifacts.zip_sha256),
                versions::zip_json_sha256.eq(artifacts.manifest_sha256),
            ))
            .execute(&mut conn)
            .await
            .context("Failed to save zip checksums to the database")?;

        info!(
            duration = start.elapsed().as_nanos(),
            "Zip build completed for {name}@{version} (version_id={version_id})"
        );

        Ok(())
    }
}

/// Crate name, version number, and publish time for a given version ID.
#[derive(Debug, HasQuery)]
#[diesel(
    table_name = versions,
    base_query = versions::table.inner_join(crates::table)
)]
struct CrateVersionInfo {
    #[diesel(select_expression = crates::columns::name)]
    name: String,
    #[diesel(select_expression = versions::columns::num)]
    version: String,
    created_at: DateTime<Utc>,
}

impl CrateVersionInfo {
    /// Looks up the info for a given version ID.
    #[instrument(skip(conn))]
    async fn load(version_id: i32, mut conn: &AsyncPgConnection) -> anyhow::Result<Option<Self>> {
        Self::query()
            .filter(versions::id.eq(version_id))
            .first(&mut conn)
            .await
            .optional()
            .context("Failed to query crate and version info")
    }

    /// Converts the publish time to a zip entry timestamp.
    fn created_at(&self) -> anyhow::Result<crates_io_crate_zip::DateTime> {
        crates_io_crate_zip::DateTime::from_date_and_time(
            self.created_at.year() as u16,
            self.created_at.month() as u8,
            self.created_at.day() as u8,
            self.created_at.hour() as u8,
            self.created_at.minute() as u8,
            self.created_at.second() as u8,
        )
        .context("Failed to build zip entry timestamp from publish time")
    }
}

/// Downloads the `.crate` tarball into a temporary file and returns it.
#[instrument(skip(storage))]
async fn download_to_tempfile(
    storage: &crate::storage::Storage,
    krate: &str,
    version: &str,
) -> anyhow::Result<File> {
    let file = tempfile::tempfile().context("Failed to create temporary file")?;
    let mut writer = tokio::fs::File::from_std(file);

    let key = StorageKey::for_crate_file(krate, version);
    let mut stream = storage
        .download_crate_file(&key)
        .await
        .context("Failed to download crate file")?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Failed to read crate file chunk")?;
        writer
            .write_all(&chunk)
            .await
            .context("Failed to write crate file to disk")?;
    }
    writer.flush().await.context("Failed to flush crate file")?;

    Ok(writer.into_std().await)
}

/// The artifacts produced from a `.crate`, ready to upload and record.
struct Artifacts {
    zip: File,
    zip_sha256: Vec<u8>,
    manifest_json: Vec<u8>,
    manifest_sha256: Vec<u8>,
}

impl Artifacts {
    /// Builds the zip and manifest from the downloaded tarball and hashes both.
    /// Runs synchronously, so it must be called inside [`spawn_blocking()`].
    fn build(mut tarball: File, modified: crates_io_crate_zip::DateTime) -> anyhow::Result<Self> {
        let mut zip = tempfile::tempfile().context("Failed to create temporary zip file")?;

        let manifest = build_zip(&mut tarball, modified, &zip).context("Failed to build zip")?;

        zip.rewind().context("Failed to rewind zip file")?;
        let zip_sha256 = sha256(&mut zip).context("Failed to hash zip file")?;
        zip.rewind().context("Failed to rewind zip file")?;

        let manifest_json =
            serde_json::to_vec(&manifest).context("Failed to serialize manifest")?;
        let manifest_sha256 = Sha256::digest(&manifest_json).to_vec();

        Ok(Self {
            zip,
            zip_sha256,
            manifest_json,
            manifest_sha256,
        })
    }
}

/// Streams `reader` through a SHA-256 hasher and returns the raw 32-byte digest.
fn sha256(reader: &mut impl Read) -> std::io::Result<Vec<u8>> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_vec())
}
