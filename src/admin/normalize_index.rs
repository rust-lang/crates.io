use std::{
    fs::File,
    io::{BufRead, BufReader},
    process::Command,
};

use cargo_registry_index::{Repository, RepositoryConfig};
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};

use crate::admin::dialoguer;

#[derive(clap::Parser, Debug, Copy, Clone)]
#[clap(name = "normalize-index", about = "Normalize and squash the git index")]
pub struct Opts {}

pub fn run(_opts: Opts) -> anyhow::Result<()> {
    println!("fetching git repo");
    let config = RepositoryConfig::from_environment();
    let repo = Repository::open(&config)?;

    repo.reset_head()?;
    println!("please place site in read-only mode now to prevent further commits");
    if !dialoguer::confirm("continue?") {
        return Ok(());
    }
    repo.reset_head()?;
    println!("HEAD is at {}", repo.head_oid()?);

    let files = repo.get_files_modified_since(None)?;
    println!("found {} crates", files.len());
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(ProgressStyle::with_template("{bar:60} ({pos}/{len}, ETA {eta})").unwrap());

    for file in files.iter().progress_with(pb) {
        let crate_name = file.file_name().unwrap().to_str().unwrap();
        let path = repo.index_file(crate_name);
        if !path.exists() {
            continue;
        }

        let mut body: Vec<u8> = Vec::new();
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let mut versions = Vec::new();
        for line in reader.lines() {
            let mut krate: cargo_registry_index::Crate = serde_json::from_str(&line?)?;
            for dep in &mut krate.deps {
                // Remove deps with empty features
                dep.features.retain(|d| !d.is_empty());
                // Set null DependencyKind to Normal
                dep.kind = Some(
                    dep.kind
                        .unwrap_or(cargo_registry_index::DependencyKind::Normal),
                );
            }
            krate.deps.sort();
            versions.push(krate);
        }
        for version in versions {
            serde_json::to_writer(&mut body, &version).unwrap();
            body.push(b'\n');
        }
        std::fs::write(path, body)?;
    }

    println!("committing normalization");
    let msg = "Normalize index format\n\n\
        More information can be found at https://github.com/rust-lang/crates.io/pull/5066";
    repo.run_command(Command::new("git").args(["commit", "-am", msg]))?;

    if dialoguer::confirm("push to origin?") {
        repo.run_command(Command::new("git").args(["push", "origin"]))?;
        println!("The index has been successfully normalized.");
    }
    Ok(())
}
