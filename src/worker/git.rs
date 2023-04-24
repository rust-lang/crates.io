use crate::background_jobs::{Environment, Job, NormalizeIndexJob};
use crate::models;
use crate::swirl::PerformError;
use anyhow::Context;
use cargo_registry_index::{Crate, Repository};
use chrono::Utc;
use diesel::prelude::*;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::process::Command;

#[instrument(skip_all, fields(krate.name = ?krate.name, krate.vers = ?krate.vers))]
pub fn perform_index_add_crate(
    env: &Environment,
    conn: &mut PgConnection,
    krate: &Crate,
) -> Result<(), PerformError> {
    info!("Adding {}#{} to the git index", krate.name, krate.vers);

    use std::io::prelude::*;

    let repo = env.lock_index()?;
    let dst = repo.index_file(&krate.name);

    // Add the crate to its relevant file
    fs::create_dir_all(dst.parent().unwrap())?;
    let mut file = OpenOptions::new().append(true).create(true).open(&dst)?;
    serde_json::to_writer(&mut file, &krate)?;
    file.write_all(b"\n")?;

    let message: String = format!("Update crate `{}#{}`", krate.name, krate.vers);
    repo.commit_and_push(&message, &dst)?;

    // Queue another background job to update the http-based index as well.
    Job::update_crate_index(krate.name.clone()).enqueue(conn)?;
    Ok(())
}

#[instrument(skip(env))]
pub fn perform_index_sync_to_http(
    env: &Environment,
    crate_name: String,
) -> Result<(), PerformError> {
    info!("Syncing git index to HTTP-based index");

    let repo = env.lock_index()?;
    let dst = repo.index_file(&crate_name);

    let contents = match fs::read_to_string(dst) {
        Ok(contents) => Some(contents),
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => return Err(e.into()),
    };

    env.uploader
        .sync_index(env.http_client(), &crate_name, contents)?;

    if let Some(cloudfront) = env.cloudfront() {
        let path = Repository::relative_index_file_for_url(&crate_name);
        info!(%path, "Invalidating index file on CloudFront");
        cloudfront.invalidate(env.http_client(), &path)?;
    }

    Ok(())
}

/// Regenerates or removes an index file for a single crate
#[instrument(skip_all, fields(krate.name = ?krate))]
pub fn sync_to_git_index(
    env: &Environment,
    conn: &mut PgConnection,
    krate: &str,
) -> Result<(), PerformError> {
    info!("Syncing to git index");

    let new = get_index_data(krate, conn).context("Failed to get index data")?;

    let repo = env.lock_index()?;
    let dst = repo.index_file(krate);

    // Read the previous crate contents
    let old = match fs::read_to_string(&dst) {
        Ok(content) => Some(content),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };

    match (old, new) {
        (None, Some(new)) => {
            fs::create_dir_all(dst.parent().unwrap())?;
            let mut file = File::create(&dst)?;
            file.write_all(new.as_bytes())?;
            repo.commit_and_push(&format!("Create crate `{}`", krate), &dst)?;
        }
        (Some(old), Some(new)) if old != new => {
            let mut file = File::create(&dst)?;
            file.write_all(new.as_bytes())?;
            repo.commit_and_push(&format!("Update crate `{}`", krate), &dst)?;
        }
        (Some(_old), None) => {
            fs::remove_file(&dst)?;
            repo.commit_and_push(&format!("Delete crate `{}`", krate), &dst)?;
        }
        _ => debug!("Skipping sync because index is up-to-date"),
    }

    Ok(())
}

/// Regenerates or removes an index file for a single crate
#[instrument(skip_all, fields(krate.name = ?krate))]
pub fn sync_to_sparse_index(
    env: &Environment,
    conn: &mut PgConnection,
    krate: &str,
) -> Result<(), PerformError> {
    info!("Syncing to sparse index");

    let content = get_index_data(krate, conn).context("Failed to get index data")?;

    env.uploader
        .sync_index(env.http_client(), krate, content)
        .context("Failed to sync index data")?;

    if let Some(cloudfront) = env.cloudfront() {
        let path = Repository::relative_index_file_for_url(krate);

        info!(%path, "Invalidating index file on CloudFront");
        cloudfront
            .invalidate(env.http_client(), &path)
            .context("Failed to invalidate CloudFront")?;
    }

    Ok(())
}

#[instrument(skip_all, fields(krate.name = ?name))]
pub fn get_index_data(name: &str, conn: &mut PgConnection) -> anyhow::Result<Option<String>> {
    debug!("Looking up crate by name");
    let Some(krate): Option<models::Crate> = models::Crate::by_exact_name(name).first(conn).optional()? else {
        return Ok(None);
    };

    debug!("Gathering remaining index data");
    let crates = krate
        .index_metadata(conn)
        .context("Failed to gather index metadata")?;

    debug!("Serializing index data");
    let mut bytes = Vec::new();
    cargo_registry_index::write_crates(&crates, &mut bytes)
        .context("Failed to serialize index metadata")?;

    let str = String::from_utf8(bytes).context("Failed to decode index metadata as utf8")?;

    Ok(Some(str))
}

/// Collapse the index into a single commit, archiving the current history in a snapshot branch.
#[instrument(skip(env))]
pub fn perform_index_squash(env: &Environment) -> Result<(), PerformError> {
    info!("Squashing the index into a single commit");

    let repo = env.lock_index()?;

    let now = Utc::now().format("%Y-%m-%d");
    let original_head = repo.head_oid()?.to_string();
    let msg = format!("Collapse index into one commit\n\n\
        Previous HEAD was {original_head}, now on the `snapshot-{now}` branch\n\n\
        More information about this change can be found [online] and on [this issue].\n\n\
        [online]: https://internals.rust-lang.org/t/cargos-crate-index-upcoming-squash-into-one-commit/8440\n\
        [this issue]: https://github.com/rust-lang/crates-io-cargo-teams/issues/47");

    repo.squash_to_single_commit(&msg)?;

    // Shell out to git because libgit2 does not currently support push leases

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
        &format!("{original_head}:refs/heads/snapshot-{now}"),
    ]))?;

    info!("The index has been successfully squashed.");

    Ok(())
}

pub fn perform_normalize_index(
    env: &Environment,
    args: NormalizeIndexJob,
) -> Result<(), PerformError> {
    info!("Normalizing the index");

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
        fs::write(path, body)?;
    }

    info!("Committing normalization");
    let msg = "Normalize index format\n\n\
        More information can be found at https://github.com/rust-lang/crates.io/pull/5066";
    repo.run_command(Command::new("git").args(["commit", "-am", msg]))?;

    let branch = match args.dry_run {
        false => "master",
        true => "normalization-dry-run",
    };

    info!(?branch, "Pushing to upstream repository");
    repo.run_command(Command::new("git").args(["push", "origin", &format!("HEAD:{branch}")]))?;

    info!("Index normalization completed");

    Ok(())
}
