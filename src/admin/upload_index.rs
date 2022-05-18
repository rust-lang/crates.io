use std::time::{Duration, Instant};

use crate::admin::dialoguer;
use cargo_registry_index::{Repository, RepositoryConfig};
use reqwest::blocking::Client;

use crate::config;

#[derive(clap::Parser, Debug)]
#[clap(
    name = "upload-index",
    about = "Upload index from git to S3 (http-based index)"
)]
pub struct Opts {
    /// Incremental commit. Any changed files made after this commit will be uploaded.
    incremental_commit: Option<String>,
}

pub fn run(opts: Opts) -> anyhow::Result<()> {
    let config = config::Base::from_environment();
    let uploader = config.uploader();
    let client = Client::new();

    println!("fetching git repo");
    let config = RepositoryConfig::from_environment();
    let repo = Repository::open(&config)?;
    repo.reset_head()?;
    println!("HEAD is at {}", repo.head_oid()?);

    let files = repo.get_files_modified_since(opts.incremental_commit.as_deref())?;
    println!("found {} files to upload", files.len());
    if !dialoguer::confirm("continue with upload?") {
        return Ok(());
    }

    let mut progress_update_time = Instant::now();
    for (i, file) in files.iter().enumerate() {
        let crate_name = file.file_name().unwrap().to_str().unwrap();
        let path = repo.index_file(crate_name);
        if !path.exists() {
            println!("skipping file `{}`", crate_name);
            continue;
        }
        let contents = std::fs::read_to_string(&path)?;
        uploader.upload_index(&client, crate_name, contents)?;

        // Print a progress update every 10 seconds.
        let now = Instant::now();
        if now - progress_update_time > Duration::from_secs(10) {
            progress_update_time = now;
            println!("uploading {}/{}", i, files.len());
        }
    }

    println!(
        "uploading completed; use `upload-index {}` for an incremental run",
        repo.head_oid()?
    );
    Ok(())
}
