use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use crates_io_index::Crate;
use crates_io_worker::BackgroundJob;
use std::fs;
use std::io::{BufRead, BufReader};
use std::process::Command;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct NormalizeIndex {
    dry_run: bool,
}

impl NormalizeIndex {
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }
}

impl BackgroundJob for NormalizeIndex {
    const JOB_NAME: &'static str = "normalize_index";
    const QUEUE: &'static str = "repository";

    type Context = Arc<Environment>;

    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        info!("Normalizing the index");

        let dry_run = self.dry_run;
        spawn_blocking(move || {
            let repo = env.lock_index()?;

            let files = repo.get_files_modified_since(None)?;
            let num_files = files.len();

            for (i, file) in files.iter().enumerate() {
                if i % 50 == 0 {
                    info!(num_files, i, ?file);
                }

                let crate_name = file.file_name().unwrap().to_str().unwrap();
                let path = repo.index_file(crate_name);
                if !path.exists() {
                    continue;
                }

                let mut body: Vec<u8> = Vec::new();
                let file = fs::File::open(&path)?;
                let reader = BufReader::new(file);
                let mut versions = Vec::new();
                for line in reader.lines() {
                    let line = line?;
                    if line.is_empty() {
                        continue;
                    }

                    let mut krate: Crate = serde_json::from_str(&line)?;
                    for dep in &mut krate.deps {
                        // Remove deps with empty features
                        dep.features.retain(|d| !d.is_empty());
                        // Set null DependencyKind to Normal
                        dep.kind =
                            Some(dep.kind.unwrap_or(crates_io_index::DependencyKind::Normal));
                    }
                    krate.deps.sort();
                    versions.push(krate);
                }
                for version in versions {
                    serde_json::to_writer(&mut body, &version).unwrap();
                    body.push(b'\n');
                }
                fs::write(path, body)?;
            }

            info!("Committing normalization");
            let msg = "Normalize index format\n\n\
        More information can be found at https://github.com/rust-lang/crates.io/pull/5066";
            repo.run_command(Command::new("git").args(["commit", "-am", msg]))?;

            let branch = match dry_run {
                false => "master",
                true => "normalization-dry-run",
            };

            info!(?branch, "Pushing to upstream repository");
            repo.run_command(Command::new("git").args([
                "push",
                "origin",
                &format!("HEAD:{branch}"),
            ]))?;

            info!("Index normalization completed");

            Ok(())
        })
        .await?
    }
}
