use anyhow::anyhow;
use clap::Parser;
use crates_io_tarball::{TarballCompatibility, TarballPolicy, process_tarball};
use futures_util::{StreamExt, stream};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use walkdir::WalkDir;

/// Runs through all crate files in a folder and shows parsing errors.
#[derive(clap::Parser, Debug, Clone)]
pub struct Options {
    /// Path to the folder to scan for crate files
    path: PathBuf,

    /// Number of CPUs to use for scanning and processing crate files
    #[arg(short, long, default_value_t = default_jobs())]
    jobs: NonZeroUsize,
}

/// Returns the number of CPUs available to this process, falling back to 1.
fn default_jobs() -> NonZeroUsize {
    std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN)
}

fn main() -> anyhow::Result<()> {
    setup_tracing();

    let options = Options::parse();
    let jobs = options.jobs.get();

    rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build_global()?;

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(jobs)
        .enable_all()
        .build()?
        .block_on(run(options.path, jobs))
}

/// Scans `path` for crate files and processes them using `jobs` worker threads.
async fn run(path: PathBuf, jobs: usize) -> anyhow::Result<()> {
    if !path.is_dir() {
        return Err(anyhow!("`{}` not found or not a directory", path.display()));
    }

    info!(path = %path.display(), "Searching for crate files");

    let pb = ProgressBar::new(u64::MAX);
    pb.set_style(ProgressStyle::with_template(
        "{human_pos} crate files found",
    )?);

    let mut paths = WalkDir::new(path)
        .into_iter()
        .par_bridge()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.into_path())
        .filter(|path| path.is_file() && path.extension().unwrap_or_default() == "crate")
        .progress_with(pb)
        .collect::<Vec<_>>();

    paths.par_sort();

    let num_files = paths.len();
    info!(%num_files, "Processing crate files");

    let pb = ProgressBar::new(num_files as u64);
    pb.set_style(ProgressStyle::with_template(
        "{bar:60} ({pos}/{len}, ETA {eta}) {wide_msg}",
    )?);

    // Spawn each crate file onto its own task so decompression and parsing are
    // spread across the runtime's worker threads, and bound the number of
    // in-flight files to avoid exhausting file descriptors on large corpora.
    let concurrency = jobs * 4;

    stream::iter(paths)
        .map(|path| {
            let pb = pb.clone();
            tokio::spawn(async move { process_path(&path, &pb).await })
        })
        .buffer_unordered(concurrency)
        .for_each(|result| {
            if let Err(error) = result {
                pb.suspend(|| warn!("Crate processing task failed: {error}"));
            }
            pb.inc(1);
            async {}
        })
        .await;

    pb.finish();

    Ok(())
}

async fn process_path(path: &Path, pb: &ProgressBar) {
    let file = File::open(path)
        .await
        .map_err(|error| pb.suspend(|| warn!("Failed to read crate file: {error}")));

    let Ok(mut file) = file else {
        return;
    };

    let path_no_ext = path.with_extension("");
    let pkg_name = path_no_ext.file_name().unwrap().to_string_lossy();
    pb.set_message(format!("{pkg_name}"));

    let policy = TarballPolicy {
        unpack_size: u64::MAX,
        metadata_file_size: u64::MAX,
        entries: None,
        compatibility: TarballCompatibility::Legacy,
    };
    let result = process_tarball(&pkg_name, &mut file, policy).await;
    pb.suspend(|| match result {
        Ok(result) => debug!(%pkg_name, path = %path.display(), ?result),
        Err(error) => {
            // Convert to anyhow so `{error:#}` includes the source chain.
            let error = anyhow::Error::new(error);
            warn!(%pkg_name, path = %path.display(), "Failed to process tarball: {error:#}")
        }
    })
}

fn setup_tracing() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .without_time()
        .with_target(false)
        .init();
}
