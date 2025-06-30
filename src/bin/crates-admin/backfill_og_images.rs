use anyhow::Result;
use crates_io::db;
use crates_io::schema::{background_jobs, crates};
use crates_io::worker::jobs::GenerateOgImage;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tracing::{info, warn};

#[derive(clap::Parser, Debug)]
#[command(
    name = "backfill-og-images",
    about = "Enqueue OG image generation jobs for existing crates"
)]
pub struct Opts {
    #[arg(long, default_value = "1000")]
    /// Batch size for enqueueing crates (default: 1000)
    batch_size: usize,

    #[arg(long)]
    /// Only generate OG images for crates with names starting with this prefix
    prefix: Option<String>,

    #[arg(long)]
    /// Offset to start enqueueing from (useful for resuming)
    offset: Option<i64>,
}

pub async fn run(opts: Opts) -> Result<()> {
    let mut conn = db::oneoff_connection().await?;

    info!("Starting OG image backfill with options: {opts:?}");

    // Helper function to build query
    let build_query = |offset: i64| {
        let mut query = crates::table
            .select(crates::name)
            .order(crates::name)
            .into_boxed();

        if let Some(prefix) = &opts.prefix {
            query = query.filter(crates::name.like(format!("{prefix}%")));
        }

        query.offset(offset)
    };

    // Count total crates to process
    let mut count_query = crates::table.into_boxed();
    if let Some(prefix) = &opts.prefix {
        count_query = count_query.filter(crates::name.like(format!("{prefix}%")));
    }
    let total_crates: i64 = count_query.count().get_result(&mut conn).await?;

    info!("Total crates to enqueue: {total_crates}");

    let mut offset = opts.offset.unwrap_or(0);
    let mut enqueued = 0;
    let mut errors = 0;

    loop {
        // Fetch batch of crate names
        let crate_names: Vec<String> = build_query(offset)
            .limit(opts.batch_size as i64)
            .load(&mut conn)
            .await?;

        if crate_names.is_empty() {
            break;
        }

        let batch_size = crate_names.len();
        info!(
            "Enqueueing batch {}-{} of {total_crates}",
            offset + 1,
            offset + batch_size as i64
        );

        // Create batch of jobs
        let jobs = crate_names
            .into_iter()
            .map(GenerateOgImage::new)
            .map(|job| {
                Ok((
                    background_jobs::job_type.eq(GenerateOgImage::JOB_NAME),
                    background_jobs::data.eq(serde_json::to_value(job)?),
                    background_jobs::priority.eq(-10),
                ))
            })
            .collect::<serde_json::Result<Vec<_>>>()?;

        // Batch insert all jobs
        let result = diesel::insert_into(background_jobs::table)
            .values(jobs)
            .execute(&mut conn)
            .await;

        match result {
            Ok(inserted_count) => {
                enqueued += inserted_count;
                info!("Enqueued {enqueued} jobs so far...");
            }
            Err(e) => {
                errors += batch_size;
                warn!("Failed to enqueue batch of OG image jobs: {e}");
            }
        }

        // Break if we've processed fewer than batch_size (last batch)
        if batch_size < opts.batch_size {
            break;
        }

        offset += opts.batch_size as i64;
    }

    info!("Jobs enqueued: {enqueued}");
    if errors > 0 {
        warn!("{errors} jobs failed to enqueue. Check logs above for details.");
    }

    Ok(())
}
