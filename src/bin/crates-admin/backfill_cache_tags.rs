use anyhow::Context;
use crates_io::config::FeaturesConfig;
use crates_io::db;
use crates_io::schema::background_jobs;
use crates_io::schema::{cache_tags_backfills, crates};
use crates_io::worker::jobs::BackfillCacheTags;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use indicatif::{ProgressBar, ProgressStyle};

const CHUNK_SIZE: usize = 100;

/// Priority for the queued jobs. Negative so the one-time bulk backfill yields
/// to regular background work.
const PRIORITY: i16 = -50;

#[derive(clap::Parser, Debug, Eq, PartialEq)]
#[clap(
    name = "backfill-cache-tags",
    about = "Queue background jobs to backfill `cache-tags` metadata onto a crate's S3 objects.",
    group(clap::ArgGroup::new("mode").required(true))
)]
pub struct Options {
    /// Backfill *all* crates that have not yet been backfilled.
    #[clap(long, group = "mode")]
    backfill: bool,

    /// Names of the crates to backfill.
    #[clap(group = "mode")]
    crates: Vec<String>,
}

pub async fn run(opts: Options) -> anyhow::Result<()> {
    let features = FeaturesConfig::from_env().context("Failed to load features config")?;
    if !features.cache_tags_enabled {
        println!("`CACHE_TAGS_ENABLED` is not set, skipping backfill");
        return Ok(());
    }

    let conn = db::oneoff_connection().await;
    let mut conn = conn.context("Failed to connect to the database")?;

    let names = load_crate_names(&opts, &conn).await;
    let names = names.context("Failed to load crates")?;
    if names.is_empty() {
        println!("No matching crates found.");
        return Ok(());
    }

    println!("Found {} matching crates", names.len());

    let pb_style = ProgressStyle::with_template("{bar:60} ({pos}/{len}, ETA {eta})")?;
    let pb = ProgressBar::new(names.len() as u64).with_style(pb_style);

    let mut queued_count = 0;
    let mut error_count = 0;

    for batch in names.chunks(CHUNK_SIZE) {
        let jobs = batch
            .iter()
            .map(|name| NewBackgroundJob::new(name))
            .collect::<anyhow::Result<Vec<_>>>()?;

        let num_jobs = jobs.len();

        let result = diesel::insert_into(background_jobs::table)
            .values(&jobs)
            .execute(&mut conn)
            .await;

        pb.inc(num_jobs as u64);

        if let Err(err) = result {
            error_count += num_jobs;
            pb.suspend(|| eprintln!("Failed to queue background jobs: {err}"));
        } else {
            queued_count += num_jobs;
        }
    }

    pb.finish_with_message("Done");

    println!("Successfully queued {queued_count} jobs");
    if error_count > 0 {
        println!("Failed to queue {error_count} jobs");
    }

    Ok(())
}

async fn load_crate_names(
    opts: &Options,
    mut conn: &AsyncPgConnection,
) -> QueryResult<Vec<String>> {
    let mut query = crates::table
        .left_join(
            cache_tags_backfills::table
                .on(cache_tags_backfills::crate_id.eq(crates::id.nullable())),
        )
        .select(crates::name)
        .into_boxed();

    if opts.backfill {
        query = query.filter(cache_tags_backfills::id.is_null());
    } else {
        query = query.filter(crates::name.eq_any(&opts.crates));
    }

    query.load(&mut conn).await
}

/// Represents a new background job to be inserted into the database.
#[derive(Debug, diesel::Insertable)]
#[diesel(table_name = background_jobs)]
struct NewBackgroundJob {
    job_type: &'static str,
    data: serde_json::Value,
    priority: i16,
}

impl NewBackgroundJob {
    /// Creates a new [`BackfillCacheTags`] background job for the given crate.
    fn new(name: &str) -> anyhow::Result<Self> {
        let job = BackfillCacheTags::new(name.to_string());
        let data = serde_json::to_value(&job).context("Failed to serialize job data")?;

        Ok(Self {
            job_type: BackfillCacheTags::JOB_NAME,
            data,
            priority: PRIORITY,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crates_io::schema::cache_tags_backfills;
    use crates_io_database::models::{NewCacheTagsBackfillRow, NewUser};
    use crates_io_test_db::TestDatabase;
    use crates_io_test_utils::builders::CrateBuilder;

    async fn create_user(conn: &AsyncPgConnection) -> i32 {
        NewUser::builder()
            .gh_id(1)
            .gh_login("testuser")
            .username("testuser")
            .gh_encrypted_token(&[])
            .build()
            .insert(conn)
            .await
            .unwrap()
    }

    fn opts(backfill: bool, crates: &[&str]) -> Options {
        Options {
            backfill,
            crates: crates.iter().map(|name| name.to_string()).collect(),
        }
    }

    #[tokio::test]
    async fn load_crate_names_returns_named_crates() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;
        let user_id = create_user(&conn).await;

        CrateBuilder::new("foo", user_id)
            .expect_build(&mut conn)
            .await;

        CrateBuilder::new("bar", user_id)
            .expect_build(&mut conn)
            .await;

        let names = load_crate_names(&opts(false, &["foo"]), &conn)
            .await
            .unwrap();

        assert_eq!(names, vec!["foo"]);
    }

    #[tokio::test]
    async fn load_crate_names_backfill_excludes_completed_crates() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;
        let user_id = create_user(&conn).await;

        let foo = CrateBuilder::new("foo", user_id)
            .expect_build(&mut conn)
            .await;

        CrateBuilder::new("bar", user_id)
            .expect_build(&mut conn)
            .await;

        let row = NewCacheTagsBackfillRow::builder()
            .crate_id(foo.id)
            .crate_name("foo")
            .build();

        diesel::insert_into(cache_tags_backfills::table)
            .values(row)
            .execute(&mut conn)
            .await
            .unwrap();

        let names = load_crate_names(&opts(true, &[]), &conn).await.unwrap();
        assert_eq!(names, vec!["bar"]);
    }
}
