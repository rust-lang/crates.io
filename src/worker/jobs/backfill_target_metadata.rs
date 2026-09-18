use crate::schema::{crates, versions};
use crate::storage::{Storage, StorageKey};
use crate::worker::WorkerContext;
use anyhow::Context;
use crates_io_cargo_toml::Manifest as CargoManifest;
use crates_io_cargo_toml::target_metadata::{ErrorStatus, TargetMetadata, TargetMetadataAnalysis};
use crates_io_crate_zip::Manifest as ZipManifest;
use crates_io_database::models::VersionTargetMetadata;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};

/// Collects target metadata for one previously published crate version.
#[derive(Clone, Serialize, Deserialize)]
pub struct BackfillTargetMetadata {
    version_id: i32,
}

impl BackfillTargetMetadata {
    /// Creates a target metadata backfill job for the given version.
    pub fn new(version_id: i32) -> Self {
        Self { version_id }
    }
}

impl BackgroundJob for BackfillTargetMetadata {
    const JOB_NAME: &'static str = "backfill_target_metadata";
    const QUEUE: &'static str = "backfill";
    const DEDUPLICATED: bool = true;

    type Context = WorkerContext;

    #[instrument(skip_all, fields(version_id = self.version_id))]
    async fn run(self, ctx: Self::Context) -> anyhow::Result<()> {
        let info = {
            let conn = ctx.deadpool.get().await?;
            CrateVersionInfo::load(self.version_id, &conn).await?
        };
        let Some(info) = info else {
            warn!("Version not found, skipping target metadata backfill");
            return Ok(());
        };

        let analysis = match analyze(&ctx.storage, &info.name, &info.version).await {
            Ok(metadata) => {
                if let Err(error) = metadata.require_existing_sources() {
                    warn!("Target metadata validation failed: {error}");
                }
                TargetMetadataAnalysis::Success(metadata)
            }
            Err(AnalysisError::Storage(error)) => return Err(error),
            Err(AnalysisError::Artifact(error)) => {
                warn!("Target metadata analysis failed permanently: {error:#}");
                TargetMetadataAnalysis::Error {
                    status: ErrorStatus::Error,
                }
            }
        };

        let metadata = VersionTargetMetadata(analysis);
        let mut conn = ctx.deadpool.get().await?;
        diesel::update(versions::table.find(self.version_id))
            .set(versions::target_metadata.eq(Some(&metadata)))
            .execute(&mut conn)
            .await
            .context("Failed to save target metadata")?;

        Ok(())
    }
}

/// Crate name and version number for a given version ID.
#[derive(Debug, HasQuery)]
#[diesel(base_query = versions::table.inner_join(crates::table))]
struct CrateVersionInfo {
    #[diesel(select_expression = crates::columns::name)]
    name: String,
    #[diesel(select_expression = versions::columns::num)]
    version: String,
}

impl CrateVersionInfo {
    /// Looks up the info for a given version ID.
    async fn load(version_id: i32, mut conn: &AsyncPgConnection) -> anyhow::Result<Option<Self>> {
        Self::query()
            .filter(versions::id.eq(version_id))
            .first(&mut conn)
            .await
            .optional()
            .context("Failed to query crate version")
    }
}

#[derive(Debug, thiserror::Error)]
enum AnalysisError {
    #[error(transparent)]
    Storage(anyhow::Error),
    #[error(transparent)]
    Artifact(anyhow::Error),
}

async fn analyze(
    storage: &Storage,
    name: &str,
    version: &str,
) -> Result<TargetMetadata, AnalysisError> {
    let manifest_key = StorageKey::for_crate_zip_manifest(name, version);
    let manifest = storage
        .download(&manifest_key)
        .await
        .with_context(|| format!("Failed to download ZIP manifest for {name}@{version}"))
        .map_err(AnalysisError::Storage)?;
    let manifest: ZipManifest = serde_json::from_slice(&manifest)
        .with_context(|| format!("Failed to parse ZIP manifest for {name}@{version}"))
        .map_err(AnalysisError::Artifact)?;
    let cargo_toml = manifest
        .cargo_toml()
        .with_context(|| format!("Failed to locate `Cargo.toml` for {name}@{version}"))
        .map_err(AnalysisError::Artifact)?;
    let range = cargo_toml
        .data_range()
        .with_context(|| format!("Invalid `Cargo.toml` range for {name}@{version}"))
        .map_err(AnalysisError::Artifact)?;

    let zip_key = StorageKey::for_crate_zip(name, version);
    let compressed = storage
        .download_range(&zip_key, range)
        .await
        .with_context(|| format!("Failed to download `Cargo.toml` for {name}@{version}"))
        .map_err(AnalysisError::Storage)?;
    let contents = cargo_toml
        .decode(&compressed)
        .with_context(|| format!("Failed to decode `Cargo.toml` for {name}@{version}"))
        .map_err(AnalysisError::Artifact)?;
    let mut cargo_toml = CargoManifest::from_slice(&contents)
        .with_context(|| format!("Failed to parse `Cargo.toml` for {name}@{version}"))
        .map_err(AnalysisError::Artifact)?;

    let files = manifest.files.iter().map(|file| file.path.as_str());
    TargetMetadata::extract(&mut cargo_toml, files)
        .with_context(|| format!("Failed to extract target metadata for {name}@{version}"))
        .map_err(AnalysisError::Artifact)
}
