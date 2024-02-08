use anyhow::Context;
use clap::Parser;
use crates_io_cdn_logs::{count_downloads, Decompressor};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::fs::File;
use tokio::io::BufReader;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Debug, clap::Parser)]
struct Options {
    /// The path to the CDN log file to parse
    path: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let options = Options::parse();

    let file = File::open(&options.path)
        .await
        .with_context(|| format!("Failed to open {}", options.path.display()))?;

    let reader = BufReader::new(file);

    let extension = options
        .path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();

    let start = SystemTime::now();
    let downloads = match extension {
        "gz" | "zst" => {
            let decompressor = Decompressor::from_extension(reader, Some(extension))?;
            let reader = BufReader::new(decompressor);
            count_downloads(reader).await?
        }
        _ => count_downloads(reader).await?,
    };
    let duration = start.elapsed()?;
    println!("{downloads:?}");
    println!();

    let num_crates = downloads
        .iter()
        .map(|((_, krate, _), _)| krate)
        .collect::<HashSet<_>>()
        .len();

    let total_inserts = downloads.len();

    let total_downloads = downloads
        .iter()
        .map(|(_, downloads)| downloads)
        .sum::<u64>();

    println!("Number of crates: {num_crates}");
    println!("Number of needed inserts: {total_inserts}");
    println!("Total number of downloads: {total_downloads}");
    println!("Time to parse: {duration:?}");

    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    fmt().compact().with_env_filter(env_filter).init();
}
