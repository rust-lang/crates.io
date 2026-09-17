use crate::schema::{crates, versions};
use crate::storage::{Storage, StorageKey};
use crate::worker::Environment;
use anyhow::Context;
use crates_io_cargo_toml::Manifest as CargoManifest;
use crates_io_cargo_toml::target_metadata::TargetMetadata;
use crates_io_crate_zip::Manifest as ZipManifest;
use crates_io_database::models::VersionTargetMetadata;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    #[instrument(skip_all, fields(version_id = self.version_id))]
    async fn run(self, env: Self::Context) -> anyhow::Result<()> {
        let info = {
            let conn = env.deadpool.get().await?;
            CrateVersionInfo::load(self.version_id, &conn).await?
        };
        let Some(info) = info else {
            warn!("Version not found, skipping target metadata backfill");
            return Ok(());
        };

        let metadata = analyze(&env.storage, &info.name, &info.version).await?;
        if let Err(error) = metadata.require_existing_sources() {
            warn!("Target metadata validation failed: {error}");
        }

        let metadata = VersionTargetMetadata(metadata);
        let mut conn = env.deadpool.get().await?;
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

async fn analyze(storage: &Storage, name: &str, version: &str) -> anyhow::Result<TargetMetadata> {
    let manifest_key = StorageKey::for_crate_zip_manifest(name, version);
    let manifest = storage
        .download(&manifest_key)
        .await
        .with_context(|| format!("Failed to download ZIP manifest for {name}@{version}"))?;
    let manifest: ZipManifest = serde_json::from_slice(&manifest)
        .with_context(|| format!("Failed to parse ZIP manifest for {name}@{version}"))?;
    let cargo_toml = manifest
        .cargo_toml()
        .with_context(|| format!("Failed to locate `Cargo.toml` for {name}@{version}"))?;
    let range = cargo_toml
        .data_range()
        .with_context(|| format!("Invalid `Cargo.toml` range for {name}@{version}"))?;

    let zip_key = StorageKey::for_crate_zip(name, version);
    let compressed = storage
        .download_range(&zip_key, range)
        .await
        .with_context(|| format!("Failed to download `Cargo.toml` for {name}@{version}"))?;
    let contents = cargo_toml
        .decode(&compressed)
        .with_context(|| format!("Failed to decode `Cargo.toml` for {name}@{version}"))?;
    let mut cargo_toml = CargoManifest::from_slice(&contents)
        .with_context(|| format!("Failed to parse `Cargo.toml` for {name}@{version}"))?;

    let files = manifest.files.iter().map(|file| file.path.as_str());
    TargetMetadata::extract(&mut cargo_toml, files)
        .with_context(|| format!("Failed to extract target metadata for {name}@{version}"))
}
