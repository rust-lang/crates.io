use crate::dialoguer;
use anyhow::Result;
use chrono::{NaiveDate, NaiveTime, TimeZone, Utc};
use clap::builder::ArgAction;
use crates_io::db;
use crates_io::schema::crates;
use crates_io::worker::jobs;
use crates_io_database::fns::canon_crate_name;
use crates_io_worker::BackgroundJob;
use crates_io_worker::schema::background_jobs;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use std::collections::HashMap;
use tracing::warn;

const BATCH_SIZE: usize = 1000;

async fn enqueue_jobs<T: BackgroundJob>(
    conn: &mut AsyncPgConnection,
    jobs: &[T],
    priority: i16,
) -> Result<()> {
    for chunk in jobs.chunks(BATCH_SIZE) {
        let values = chunk
            .iter()
            .map(|job| {
                Ok((
                    background_jobs::job_type.eq(T::JOB_NAME),
                    background_jobs::data.eq(serde_json::to_value(job)?),
                    background_jobs::priority.eq(priority),
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        diesel::insert_into(background_jobs::table)
            .values(&values)
            .execute(conn)
            .await?;
    }

    Ok(())
}

#[derive(clap::Parser, Debug)]
#[command(
    name = "sync-index",
    about = "Synchronize crate index data to git and sparse indexes"
)]
pub struct Opts {
    /// Names of the crates to synchronize
    #[arg(required_unless_present = "updated_before")]
    names: Vec<String>,

    /// Skip syncing to the git index
    #[arg(long = "no-git", action = ArgAction::SetFalse)]
    git: bool,

    /// Skip syncing to the sparse index
    #[arg(long = "no-sparse", action = ArgAction::SetFalse)]
    sparse: bool,

    /// Number of crates per bulk sync job (enables batch mode)
    #[arg(long, requires = "commit_message")]
    batch_size: Option<usize>,

    /// Commit message for bulk sync jobs
    #[arg(long, requires = "batch_size")]
    commit_message: Option<String>,

    /// Sync all crates with `updated_at` before this date (format: YYYY-MM-DD)
    #[arg(
        long,
        value_name = "DATE",
        requires = "batch_size",
        conflicts_with = "names"
    )]
    updated_before: Option<NaiveDate>,

    /// Priority for the enqueued jobs
    #[arg(long)]
    priority: Option<i16>,
}

pub async fn run(opts: Opts) -> Result<()> {
    let mut conn = db::oneoff_connection().await?;

    let crate_names = resolve_crate_names(&mut conn, &opts).await?;

    let num_crates = crate_names.len();

    if num_crates == 0 {
        println!("No crates to sync");
        return Ok(());
    }

    if !confirm_sync(&opts, num_crates).await? {
        return Ok(());
    }

    enqueue_sync_jobs(&mut conn, &crate_names, &opts).await
}

/// Determines which crate names to sync based on the provided options.
///
/// When `updated_before` is set, queries for all crates updated before
/// that date. Otherwise, uses the explicitly provided names, warning
/// about any that don't exist in the database.
async fn resolve_crate_names(conn: &mut AsyncPgConnection, opts: &Opts) -> Result<Vec<String>> {
    if let Some(date) = opts.updated_before {
        let datetime = Utc.from_utc_datetime(&date.and_time(NaiveTime::MIN));

        let names = crates::table
            .filter(crates::updated_at.lt(datetime))
            .select(crates::name)
            .order(crates::name)
            .load(conn)
            .await?;

        return Ok(names);
    }

    // Check which crates exist in the database. Crates that don't
    // exist will still be synced, which removes them from the index.
    let canon_names = opts.names.iter().map(|name| canon(name));
    let existing_crates: Vec<String> = crates::table
        .filter(canon_crate_name(crates::name).eq_any(canon_names))
        .select(crates::name)
        .load(conn)
        .await?;

    let by_canon: HashMap<String, String> = existing_crates
        .into_iter()
        .map(|name| (canon(&name), name))
        .collect();

    let mut crate_names = Vec::with_capacity(opts.names.len());
    for name in &opts.names {
        match by_canon.get(&canon(name)) {
            Some(stored_name) => crate_names.push(stored_name.clone()),
            None => {
                warn!(
                    "Crate `{name}` does not exist in the database and will be removed from the index."
                );
                crate_names.push(name.clone());
            }
        }
    }

    Ok(crate_names)
}

/// Normalizes a crate name the same way the database's `canon_crate_name()`
/// function does, allowing user-provided names to be matched against the
/// stored names regardless of case or `-`/`_` differences.
fn canon(name: &str) -> String {
    name.to_lowercase().replace('-', "_")
}

/// Shows a confirmation prompt when batch mode is used.
///
/// Returns `true` if the sync should proceed, `false` if the user
/// declined or no confirmation was needed but nothing to do.
async fn confirm_sync(opts: &Opts, num_crates: usize) -> Result<bool> {
    let Some(batch_size) = opts.batch_size else {
        return Ok(true);
    };

    let mut prompt_parts = Vec::new();

    if opts.git {
        let num_batches = num_crates.div_ceil(batch_size);
        prompt_parts.push(format!(
            "This will sync {num_crates} crate{} to the git index in {num_batches} batch{}.",
            if num_crates == 1 { "" } else { "s" },
            if num_batches == 1 { "" } else { "es" }
        ));
    }

    if opts.sparse {
        prompt_parts.push(format!(
            "This will enqueue {num_crates} sparse index sync job{}.",
            if num_crates == 1 { "" } else { "s" }
        ));
    }

    if prompt_parts.is_empty() {
        return Ok(true);
    }

    println!("{}", prompt_parts.join("\n"));
    dialoguer::confirm("Do you want to continue?").await
}

/// Enqueues git and/or sparse index sync jobs for the given crate names.
async fn enqueue_sync_jobs(
    conn: &mut AsyncPgConnection,
    crate_names: &[String],
    opts: &Opts,
) -> Result<()> {
    conn.transaction(async |conn| {
        // Handle git index sync
        if opts.git {
            if let Some(batch_size) = opts.batch_size
                && let Some(commit_message) = opts.commit_message.as_ref()
            {
                let priority = opts.priority.unwrap_or(jobs::BulkSyncToGitIndex::PRIORITY);

                let jobs: Vec<_> = crate_names
                    .chunks(batch_size)
                    .map(|chunk| jobs::BulkSyncToGitIndex::new(chunk.to_vec(), commit_message))
                    .collect();

                println!("Enqueueing {} BulkSyncToGitIndex jobs", jobs.len());
                enqueue_jobs(conn, &jobs, priority).await?;
            } else {
                let priority = opts.priority.unwrap_or(jobs::SyncToGitIndex::PRIORITY);
                let jobs: Vec<_> = crate_names.iter().map(jobs::SyncToGitIndex::new).collect();

                println!("Enqueueing {} SyncToGitIndex jobs", jobs.len());
                enqueue_jobs(conn, &jobs, priority).await?;
            }
        }

        // Handle sparse index sync (always per-crate)
        if opts.sparse {
            let priority = opts.priority.unwrap_or(jobs::SyncToSparseIndex::PRIORITY);
            let jobs: Vec<_> = crate_names
                .iter()
                .map(jobs::SyncToSparseIndex::new)
                .collect();

            println!("Enqueueing {} SyncToSparseIndex jobs", jobs.len());
            enqueue_jobs(conn, &jobs, priority).await?;
        }

        Ok(())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crates_io_database::models::NewUser;
    use crates_io_test_db::TestDatabase;
    use crates_io_test_utils::builders::CrateBuilder;
    use insta::assert_json_snapshot;
    use serde::Serialize;

    fn opts_for_names(names: &[&str]) -> Opts {
        Opts {
            names: names.iter().map(|s| s.to_string()).collect(),
            git: true,
            sparse: true,
            batch_size: None,
            commit_message: None,
            updated_before: None,
            priority: None,
        }
    }

    async fn create_user(conn: &AsyncPgConnection) -> i32 {
        NewUser {
            gh_id: 1,
            gh_login: "testuser",
            username: "testuser",
            name: None,
            gh_encrypted_token: b"token",
        }
        .insert(conn)
        .await
        .unwrap()
    }

    #[derive(HasQuery, Serialize)]
    #[diesel(
        table_name = background_jobs,
        base_query = background_jobs::table.order(background_jobs::id)
    )]
    struct Job {
        job_type: String,
        data: serde_json::Value,
    }

    async fn all_jobs(conn: &mut AsyncPgConnection) -> Vec<Job> {
        Job::query().get_results(conn).await.unwrap()
    }

    #[tokio::test]
    async fn resolve_crate_names_returns_existing_names() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;
        let user_id = create_user(&conn).await;

        CrateBuilder::new("foo", user_id)
            .expect_build(&mut conn)
            .await;
        CrateBuilder::new("bar", user_id)
            .expect_build(&mut conn)
            .await;

        let opts = opts_for_names(&["foo", "bar"]);
        let result = resolve_crate_names(&mut conn, &opts).await.unwrap();
        assert_eq!(result, vec!["foo", "bar"]);
    }

    #[tokio::test]
    async fn resolve_crate_names_includes_nonexistent_names() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;
        let user_id = create_user(&conn).await;

        CrateBuilder::new("foo", user_id)
            .expect_build(&mut conn)
            .await;

        let opts = opts_for_names(&["foo", "deleted-crate"]);
        let result = resolve_crate_names(&mut conn, &opts).await.unwrap();
        assert_eq!(result, vec!["foo", "deleted-crate"]);
    }

    #[tokio::test]
    async fn resolve_crate_names_normalizes_to_stored_name() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;
        let user_id = create_user(&conn).await;

        CrateBuilder::new("NULL", user_id)
            .expect_build(&mut conn)
            .await;
        CrateBuilder::new("foo-bar", user_id)
            .expect_build(&mut conn)
            .await;

        let opts = opts_for_names(&["null", "foo_bar"]);
        let result = resolve_crate_names(&mut conn, &opts).await.unwrap();
        assert_eq!(result, vec!["NULL", "foo-bar"]);
    }

    #[tokio::test]
    async fn resolve_crate_names_with_updated_before() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;
        let user_id = create_user(&conn).await;

        let old_date = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let recent_date = Utc.with_ymd_and_hms(2025, 6, 1, 0, 0, 0).unwrap();

        CrateBuilder::new("old-crate", user_id)
            .updated_at(old_date)
            .expect_build(&mut conn)
            .await;
        CrateBuilder::new("recent-crate", user_id)
            .updated_at(recent_date)
            .expect_build(&mut conn)
            .await;

        let opts = Opts {
            names: vec![],
            git: true,
            sparse: true,
            batch_size: Some(100),
            commit_message: Some("test".to_string()),
            updated_before: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            priority: None,
        };

        let result = resolve_crate_names(&mut conn, &opts).await.unwrap();
        assert_eq!(result, vec!["old-crate"]);
    }

    #[tokio::test]
    async fn enqueue_sync_jobs_git_and_sparse() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let names = vec!["foo".to_string(), "bar".to_string()];
        let opts = opts_for_names(&["foo", "bar"]);
        enqueue_sync_jobs(&mut conn, &names, &opts).await.unwrap();

        assert_json_snapshot!(all_jobs(&mut conn).await);
    }

    #[tokio::test]
    async fn enqueue_sync_jobs_no_git() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let names = vec!["foo".to_string()];
        let mut opts = opts_for_names(&["foo"]);
        opts.git = false;
        enqueue_sync_jobs(&mut conn, &names, &opts).await.unwrap();

        assert_json_snapshot!(all_jobs(&mut conn).await);
    }

    #[tokio::test]
    async fn enqueue_sync_jobs_no_sparse() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let names = vec!["foo".to_string()];
        let mut opts = opts_for_names(&["foo"]);
        opts.sparse = false;
        enqueue_sync_jobs(&mut conn, &names, &opts).await.unwrap();

        assert_json_snapshot!(all_jobs(&mut conn).await);
    }

    #[tokio::test]
    async fn enqueue_sync_jobs_batch_mode() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let names: Vec<String> = (0..5).map(|i| format!("crate-{i}")).collect();
        let opts = Opts {
            names: names.clone(),
            git: true,
            sparse: true,
            batch_size: Some(2),
            commit_message: Some("bulk sync".to_string()),
            updated_before: None,
            priority: None,
        };
        enqueue_sync_jobs(&mut conn, &names, &opts).await.unwrap();

        assert_json_snapshot!(all_jobs(&mut conn).await);
    }
}
