use anyhow::{anyhow, Context};
use clap::Parser;
use crates_io_tarball::async_process_tarball;
use std::path::PathBuf;
use tokio::fs::File;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::EnvFilter;

/// Read and process a `.crate` file the same way crates.io does when publishing a crate version.
/// If the crate file has no errors, the metadata that would be written to the database will be
/// output.
#[derive(clap::Parser, Debug, Clone)]
pub struct Options {
    /// Path to the `.crate` file
    path: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();

    let options = Options::parse();

    let path = options.path;
    if !path.is_file() {
        return Err(anyhow!("`{}` not found or not a file", path.display()));
    }

    let mut file = File::open(&path).await.context("Failed to read tarball")?;

    let path_no_ext = path.with_extension("");
    let pkg_name = path_no_ext.file_name().unwrap().to_string_lossy();

    let result = async_process_tarball(&pkg_name, &mut file, u64::MAX)
        .await
        .context("Failed to process tarball")?;

    println!("{result:#?}");

    Ok(())
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
