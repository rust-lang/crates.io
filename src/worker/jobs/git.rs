use crate::models;
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use anyhow::{anyhow, Context};
use chrono::Utc;
use crates_io_env_vars::var_parsed;
use crates_io_index::{Crate, Repository};
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use sentry::Level;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::process::Command;
use std::sync::Arc;
use url::Url;

#[derive(Serialize, Deserialize)]
pub struct SyncToGitIndex {
    krate: String,
}

impl SyncToGitIndex {
    pub fn new(krate: impl Into<String>) -> Self {
        let krate = krate.into();
        Self { krate }
    }
}

impl BackgroundJob for SyncToGitIndex {
    const JOB_NAME: &'static str = "sync_to_git_index";
    const PRIORITY: i16 = 100;
    const QUEUE: &'static str = "repository";

    type Context = Arc<Environment>;

    /// Regenerates or removes an index file for a single crate
    #[instrument(skip_all, fields(krate.name = ? self.krate))]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        info!("Syncing to git index");

        let crate_name = self.krate.clone();
        let conn = env.deadpool.get().await?;
        conn.interact(move |conn| {
            let new = get_index_data(&crate_name, conn).context("Failed to get index data")?;

            let repo = env.lock_index()?;
            let dst = repo.index_file(&crate_name);

            // Read the previous crate contents
            let old = match fs::read_to_string(&dst) {
                Ok(content) => Some(content),
                Err(error) if error.kind() == ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };

            match (old, new) {
                (None, Some(new)) => {
                    fs::create_dir_all(dst.parent().unwrap())?;
                    let mut file = File::create(&dst)?;
                    file.write_all(new.as_bytes())?;
                    repo.commit_and_push(&format!("Create crate `{}`", &crate_name), &dst)?;
                }
                (Some(old), Some(new)) if old != new => {
                    let mut file = File::create(&dst)?;
                    file.write_all(new.as_bytes())?;
                    repo.commit_and_push(&format!("Update crate `{}`", &crate_name), &dst)?;
                }
                (Some(_old), None) => {
                    fs::remove_file(&dst)?;
                    repo.commit_and_push(&format!("Delete crate `{}`", &crate_name), &dst)?;
                }
                _ => debug!("Skipping sync because index is up-to-date"),
            }

            Ok(())
        })
        .await
        .map_err(|err| anyhow!(err.to_string()))?
    }
}

#[derive(Serialize, Deserialize)]
pub struct SyncToSparseIndex {
    krate: String,
}

impl SyncToSparseIndex {
    pub fn new(krate: impl Into<String>) -> Self {
        let krate = krate.into();
        Self { krate }
    }
}

impl BackgroundJob for SyncToSparseIndex {
    const JOB_NAME: &'static str = "sync_to_sparse_index";
    const PRIORITY: i16 = 100;

    type Context = Arc<Environment>;

    /// Regenerates or removes an index file for a single crate
    #[instrument(skip_all, fields(krate.name = ?self.krate))]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        info!("Syncing to sparse index");

        let crate_name = self.krate.clone();
        let conn = env.deadpool.get().await?;
        let content = conn
            .interact(move |conn| get_index_data(&crate_name, conn))
            .await
            .map_err(|err| anyhow!(err.to_string()))?
            .context("Failed to get index data")?;

        let future = env.storage.sync_index(&self.krate, content);
        future.await.context("Failed to sync index data")?;

        if let Some(cloudfront) = env.cloudfront() {
            let path = Repository::relative_index_file_for_url(&self.krate);

            info!(%path, "Invalidating index file on CloudFront");
            let future = cloudfront.invalidate(&path);
            future.await.context("Failed to invalidate CloudFront")?;
        }
        Ok(())
    }
}

#[instrument(skip_all, fields(krate.name = ?name))]
pub fn get_index_data(name: &str, conn: &mut PgConnection) -> anyhow::Result<Option<String>> {
    debug!("Looking up crate by name");
    let Some(krate): Option<models::Crate> =
        models::Crate::by_exact_name(name).first(conn).optional()?
    else {
        return Ok(None);
    };

    debug!("Gathering remaining index data");
    let crates = krate
        .index_metadata(conn)
        .context("Failed to gather index metadata")?;

    // This can sometimes happen when we delete versions upon owner request
    // but don't realize that the crate is now left with no versions at all.
    //
    // In this case we will delete the crate from the index and log a warning to
    // Sentry to clean this up in the database.
    if crates.is_empty() {
        let message = format!("Crate `{name}` has no versions left");
        sentry::capture_message(&message, Level::Warning);

        return Ok(None);
    }

    debug!("Serializing index data");
    let mut bytes = Vec::new();
    crates_io_index::write_crates(&crates, &mut bytes)
        .context("Failed to serialize index metadata")?;

    let str = String::from_utf8(bytes).context("Failed to decode index metadata as utf8")?;

    Ok(Some(str))
}

#[derive(Serialize, Deserialize)]
pub struct SquashIndex;

impl BackgroundJob for SquashIndex {
    const JOB_NAME: &'static str = "squash_index";
    const QUEUE: &'static str = "repository";

    type Context = Arc<Environment>;

    /// Collapse the index into a single commit, archiving the current history in a snapshot branch.
    #[instrument(skip_all)]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        info!("Squashing the index into a single commit");

        spawn_blocking(move || {
            let repo = env.lock_index()?;

            let now = Utc::now().format("%Y-%m-%d");
            let original_head = repo.head_oid()?.to_string();
            let msg = format!("Collapse index into one commit\n\n\
            Previous HEAD was {original_head}, now on the `snapshot-{now}` branch\n\n\
            More information about this change can be found [online] and on [this issue].\n\n\
            [online]: https://internals.rust-lang.org/t/cargos-crate-index-upcoming-squash-into-one-commit/8440\n\
            [this issue]: https://github.com/rust-lang/crates-io-cargo-teams/issues/47");

            repo.squash_to_single_commit(&msg)?;

            // Shell out to git because libgit2 does not currently support push leases

            repo.run_command(Command::new("git").args([
                "push",
                // Both updates should succeed or fail together
                "--atomic",
                "origin",
                // Overwrite master, but only if it server matches the expected value
                &format!("--force-with-lease=refs/heads/master:{original_head}"),
                // The new squashed commit is pushed to master
                "HEAD:refs/heads/master",
                // The previous value of HEAD is pushed to a snapshot branch
                &format!("{original_head}:refs/heads/snapshot-{now}"),
            ]))?;

            if let Some(archive_url) = var_parsed::<Url>("GIT_ARCHIVE_REPO_URL")? {
                repo.run_command(Command::new("git").args([
                    "push",
                    archive_url.as_str(),
                    &format!("{original_head}:snapshot-{now}"),
                ]))?;
            }

            info!("The index has been successfully squashed.");

            Ok(())
        })
        .await
    }
}

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

    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        info!("Normalizing the index");

        let dry_run = self.dry_run;
        spawn_blocking(move || {
            let repo = env.lock_index()?;

            let files = repo.get_files_modified_since(None)?;
            let num_files = files.len();

            for (i, file) in files.iter().enumerate() {
                if i % 50 == 0 {
                    info!(num_files, i, ?file);
                }

                let crate_name = file.file_name().unwrap().to_str().unwrap();
                let path = repo.index_file(crate_name);
                if !path.exists() {
                    continue;
                }

                let mut body: Vec<u8> = Vec::new();
                let file = fs::File::open(&path)?;
                let reader = BufReader::new(file);
                let mut versions = Vec::new();
                for line in reader.lines() {
                    let line = line?;
                    if line.is_empty() {
                        continue;
                    }

                    let mut krate: Crate = serde_json::from_str(&line)?;
                    for dep in &mut krate.deps {
                        // Remove deps with empty features
                        dep.features.retain(|d| !d.is_empty());
                        // Set null DependencyKind to Normal
                        dep.kind =
                            Some(dep.kind.unwrap_or(crates_io_index::DependencyKind::Normal));
                    }
                    krate.deps.sort();
                    versions.push(krate);
                }
                for version in versions {
                    serde_json::to_writer(&mut body, &version).unwrap();
                    body.push(b'\n');
                }
                fs::write(path, body)?;
            }

            info!("Committing normalization");
            let msg = "Normalize index format\n\n\
        More information can be found at https://github.com/rust-lang/crates.io/pull/5066";
            repo.run_command(Command::new("git").args(["commit", "-am", msg]))?;

            let branch = match dry_run {
                false => "master",
                true => "normalization-dry-run",
            };

            info!(?branch, "Pushing to upstream repository");
            repo.run_command(Command::new("git").args([
                "push",
                "origin",
                &format!("HEAD:{branch}"),
            ]))?;

            info!("Index normalization completed");

            Ok(())
        })
        .await
    }
}
