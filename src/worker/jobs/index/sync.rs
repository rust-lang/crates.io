use crate::models;
use crate::tasks::spawn_blocking;
use crate::util::diesel::Conn;
use crate::worker::Environment;
use anyhow::Context;
use crates_io_index::Repository;
use crates_io_worker::BackgroundJob;
use diesel::{OptionalExtension, RunQueryDsl};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use sentry::Level;
use std::fs;
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::sync::Arc;

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
        spawn_blocking(move || {
            let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

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
        let content = spawn_blocking(move || {
            let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();
            get_index_data(&crate_name, conn)
        })
        .await
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
fn get_index_data(name: &str, conn: &mut impl Conn) -> anyhow::Result<Option<String>> {
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
