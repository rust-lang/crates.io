use std::{
    fs::File,
    io::{BufRead, BufReader},
    process::Command,
};

use cargo_registry_index::{Repository, RepositoryConfig};
use chrono::Utc;
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
        versions.sort_by_cached_key(|version| semver::Version::parse(&version.vers).ok());
        for version in versions {
            serde_json::to_writer(&mut body, &version).unwrap();
            body.push(b'\n');
        }
        std::fs::write(path, body)?;
    }

    let original_head = repo.head_oid()?.to_string();

    // Add an additional commit after the squash commit that normalizes the index.
    println!("committing normalization");
    let msg = "Normalize index format\n\n\
        More information can be found at https://github.com/rust-lang/crates.io/pull/5066";
    repo.run_command(Command::new("git").args(["commit", "-am", msg]))?;
    let snapshot_head = repo.head_oid()?.to_string();

    println!("squashing");
    let now = Utc::now().format("%Y-%m-%d");
    let msg = format!("Collapse index into one commit\n\n\
        Previous HEAD was {}, now on the `snapshot-{}` branch\n\n\
        More information about this change can be found [online] and on [this issue].\n\n\
        [online]: https://internals.rust-lang.org/t/cargos-crate-index-upcoming-squash-into-one-commit/8440\n\
        [this issue]: https://github.com/rust-lang/crates-io-cargo-teams/issues/47", snapshot_head, now);
    repo.squash_to_single_commit(&msg)?;

    if dialoguer::confirm("push to origin?") {
        repo.run_command(Command::new("git").args([
            "push",
            // Both updates should succeed or fail together
            "--atomic",
            "origin",
            // Overwrite master, but only if it server matches the expected value
            &format!("--force-with-lease=refs/heads/master:{original_head}"),
            // The new squashed commit is pushed to master
            "HEAD:refs/heads/master",
            // The previous value of HEAD is pushed to a snapshot branch
            &format!("{snapshot_head}:refs/heads/snapshot-{now}"),
        ]))?;
        println!("The index has been successfully normalized and squashed.");
    }
    Ok(())
}
