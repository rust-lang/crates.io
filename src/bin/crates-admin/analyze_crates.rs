use anyhow::Context;
use crates_io::db;
use crates_io::schema::background_jobs;
use crates_io::schema::{crates, default_versions, versions};
use crates_io::worker::jobs::AnalyzeCrateFile;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use indicatif::{ProgressBar, ProgressStyle};

const CHUNK_SIZE: usize = 100;

#[derive(clap::Parser, Debug, Eq, PartialEq)]
#[clap(
    name = "analyze-crates",
    about = "Queue background jobs to analyze uploaded crate file.",
    group(clap::ArgGroup::new("mode").required(true))
)]
pub struct Options {
    /// Backfill *all* versions that are missing line count statistics.
    #[clap(long, group = "mode")]
    backfill: bool,

    /// Crate specifications to analyze (format: `crate@version` or just `crate`)
    #[clap(group = "mode")]
    crates: Vec<String>,
}

pub async fn run(opts: Options) -> anyhow::Result<()> {
    let conn = db::oneoff_connection().await;
    let mut conn = conn.context("Failed to connect to the database")?;

    let results = load_versions(&opts, &mut conn).await;
    let results = results.context("Failed to load versions")?;
    if results.is_empty() {
        println!("No matching versions found.");
        return Ok(());
    }

    println!("Found {} matching versions", results.len());
    if opts.backfill {
        let default_count = results.iter().filter(|(_, c)| *c).count();
        println!("  {default_count} default versions (priority -20)");

        let regular_count = results.len() - default_count;
        println!("  {regular_count} regular versions (priority -50)");
    }

    let pb_style = ProgressStyle::with_template("{bar:60} ({pos}/{len}, ETA {eta})")?;
    let pb = ProgressBar::new(results.len() as u64).with_style(pb_style);

    let mut queued_count = 0;
    let mut error_count = 0;

    for batch in results.chunks(CHUNK_SIZE) {
        let jobs = batch
            .iter()
            .map(|(version_id, is_default_version)| {
                let priority = if *is_default_version { -20 } else { -50 };
                NewBackgroundJob::new(*version_id, priority)
            })
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

async fn load_versions(
    opts: &Options,
    conn: &mut AsyncPgConnection,
) -> QueryResult<Vec<(i32, bool)>> {
    let mut query = versions::table
        .inner_join(crates::table)
        .left_join(default_versions::table.on(default_versions::version_id.eq(versions::id)))
        .select((
            versions::id,
            default_versions::crate_id.nullable().is_not_null(),
        ))
        .into_boxed();

    if opts.backfill {
        // Backfill mode: get all versions missing linecount data
        query = query.filter(versions::linecounts.is_null())
    } else {
        // Crate-specific mode: build a dynamic query with `or_filter`
        for crate_spec in &opts.crates {
            let (krate, version) = parse_crate_spec(crate_spec);

            query = match version {
                Some(ver) => query.or_filter(crates::name.eq(krate).and(versions::num.eq(ver))),
                None => query.or_filter(crates::name.eq(krate)),
            };
        }
    }

    query.load(conn).await
}

/// Parse crate specification in the format "crate@version" or just "crate"
fn parse_crate_spec(spec: &str) -> (&str, Option<&str>) {
    if let Some((name, ver)) = spec.split_once('@') {
        (name, Some(ver))
    } else {
        (spec, None)
    }
}

/// Represents a new background job to be inserted into the database
#[derive(Debug, diesel::Insertable)]
#[diesel(table_name = background_jobs)]
struct NewBackgroundJob {
    job_type: &'static str,
    data: serde_json::Value,
    priority: i16,
}

impl NewBackgroundJob {
    /// Create a new [AnalyzeCrateFile] background job with the specified priority
    fn new(version_id: i32, priority: i16) -> anyhow::Result<Self> {
        let job = AnalyzeCrateFile::new(version_id);
        let data = serde_json::to_value(&job).context("Failed to serialize job data")?;

        Ok(Self {
            job_type: AnalyzeCrateFile::JOB_NAME,
            data,
            priority,
        })
    }
}
