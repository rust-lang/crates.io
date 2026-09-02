use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use crates_io_index::{Crate, DependencyKind};
use crates_io_worker::BackgroundJob;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Serialize, Deserialize)]
pub struct NormalizeIndex {
    dry_run: bool,
}

impl NormalizeIndex {
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }
}

impl BackgroundJob for NormalizeIndex {
    const JOB_NAME: &'static str = "normalize_index";
    const QUEUE: &'static str = "repository";

    type Context = Arc<Environment>;

    async fn run(self, env: Self::Context) -> anyhow::Result<()> {
        info!("Normalizing the index");

        let dry_run = self.dry_run;
        spawn_blocking(move || {
            let repo = env.lock_index()?;

            let entries = repo.list_entries()?;
            let num_entries = entries.len();

            let branch = if dry_run {
                "normalization-dry-run"
            } else {
                "master"
            };
            let msg = "Normalize index format\n\n\
                More information can be found at https://github.com/rust-lang/crates.io/pull/5066";

            let mut builder = repo.commit_builder_to(msg, branch)?;
            for (i, name) in entries.iter().enumerate() {
                if i % 50 == 0 {
                    info!(num_entries, i, %name);
                }

                let Some(bytes) = repo.read_entry(name)? else {
                    continue;
                };
                let normalized = normalize_entry(&bytes)?;
                if normalized != bytes {
                    builder.upsert_entry(name, &normalized)?;
                }
            }

            info!("Committing normalization");
            builder.commit_and_push()?;
            info!("Index normalization completed");

            Ok(())
        })
        .await?
    }
}

/// Parses a newline-delimited JSON index entry, applies the normalization
/// rules (strip empty feature names, default null `kind` to `Normal`, sort
/// deps), and returns the rewritten bytes.
fn normalize_entry(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut versions = Vec::new();
    for krate in serde_json::Deserializer::from_slice(bytes).into_iter::<Crate>() {
        let mut krate = krate?;
        for dep in &mut krate.deps {
            // Remove deps with empty features
            dep.features.retain(|d| !d.is_empty());
            // Set null DependencyKind to Normal
            dep.kind = Some(dep.kind.unwrap_or(DependencyKind::Normal));
        }
        krate.deps.sort();
        versions.push(krate);
    }

    let mut body = Vec::new();
    crates_io_index::write_crates(&versions, &mut body)?;
    Ok(body)
}
