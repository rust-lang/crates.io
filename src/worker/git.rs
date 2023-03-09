use crate::background_jobs::{
    Environment, FixFeatures2Job, IndexAddCrateJob, IndexSyncToHttpJob, IndexUpdateYankedJob, Job,
    NormalizeIndexJob,
};
use crate::schema;
use crate::swirl::PerformError;
use anyhow::Context;
use cargo_registry_index::{Crate, Repository};
use chrono::Utc;
use diesel::prelude::*;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind};
use std::process::Command;

#[instrument(skip_all, fields(krate.name = ?krate.name, krate.vers = ?krate.vers))]
pub fn perform_index_add_crate(
    env: &Environment,
    conn: &mut PgConnection,
    krate: &Crate,
) -> Result<(), PerformError> {
    info!("Syncing git index to HTTP-based index");

    use std::io::prelude::*;

    let repo = env.lock_index()?;
    let dst = repo.index_file(&krate.name);

    // Add the crate to its relevant file
    fs::create_dir_all(dst.parent().unwrap())?;
    let mut file = OpenOptions::new().append(true).create(true).open(&dst)?;
    serde_json::to_writer(&mut file, &krate)?;
    file.write_all(b"\n")?;

    let message: String = format!("Updating crate `{}#{}`", krate.name, krate.vers);
    repo.commit_and_push(&message, &dst)?;

    // Queue another background job to update the http-based index as well.
    update_crate_index(krate.name.clone()).enqueue(conn)?;
    Ok(())
}

pub fn add_crate(krate: Crate) -> Job {
    Job::IndexAddCrate(IndexAddCrateJob { krate })
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

pub fn update_crate_index(crate_name: String) -> Job {
    Job::IndexSyncToHttp(IndexSyncToHttpJob { crate_name })
}

/// Yanks or unyanks a crate version. This requires finding the index
/// file, deserlialise the crate from JSON, change the yank boolean to
/// `true` or `false`, write all the lines back out, and commit and
/// push the changes.
#[instrument(skip(env, conn))]
pub fn perform_index_update_yanked(
    env: &Environment,
    conn: &mut PgConnection,
    krate: &str,
    version_num: &str,
) -> Result<(), PerformError> {
    info!("Syncing yanked status from database into the index");

    debug!("Loading yanked status from database");

    let yanked: bool = schema::versions::table
        .inner_join(schema::crates::table)
        .filter(schema::crates::name.eq(&krate))
        .filter(schema::versions::num.eq(&version_num))
        .select(schema::versions::yanked)
        .get_result(conn)
        .context("Failed to load yanked status from database")?;

    debug!(yanked);

    let repo = env.lock_index()?;
    let dst = repo.index_file(krate);

    let prev = fs::read_to_string(&dst)?;
    let new = prev
        .lines()
        .map(|line| {
            let mut git_crate = serde_json::from_str::<Crate>(line)
                .map_err(|_| format!("couldn't decode: `{line}`"))?;
            if git_crate.name != krate || git_crate.vers != version_num {
                return Ok(line.to_string());
            }
            git_crate.yanked = Some(yanked);
            Ok(serde_json::to_string(&git_crate)?)
        })
        .collect::<Result<Vec<_>, PerformError>>();
    let new = new?.join("\n") + "\n";

    if new != prev {
        fs::write(&dst, new.as_bytes())?;

        let action = if yanked { "Yanking" } else { "Unyanking" };
        let message = format!("{action} crate `{krate}#{version_num}`");

        repo.commit_and_push(&message, &dst)?;
    } else {
        debug!("Skipping `yanked` update because index is up-to-date");
    }

    // Queue another background job to update the http-based index as well.
    update_crate_index(krate.to_string()).enqueue(conn)?;

    Ok(())
}

pub fn sync_yanked(krate: String, version_num: String) -> Job {
    Job::IndexUpdateYanked(IndexUpdateYankedJob { krate, version_num })
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

pub fn squash_index() -> Job {
    Job::IndexSquash
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

pub fn normalize_index(dry_run: bool) -> Job {
    Job::NormalizeIndex(NormalizeIndexJob { dry_run })
}

pub fn perform_fix_features2(env: &Environment, args: FixFeatures2Job) -> Result<(), PerformError> {
    info!("Fixing `features2` usage in the index");

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

        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        let mut versions = Vec::new();
        let mut changed = false;
        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            let mut krate: Crate = serde_json::from_str(&line)?;
            if !krate.features.is_empty() {
                if let Some(features2) = krate.features2.as_mut() {
                    features2.extend(std::mem::take(&mut krate.features));
                    changed = true;
                }
            }

            versions.push(krate);
        }

        if changed {
            let mut body: Vec<u8> = Vec::new();
            for version in versions {
                serde_json::to_writer(&mut body, &version).unwrap();
                body.push(b'\n');
            }
            fs::write(path, body)?;
        }
    }

    info!("Committing `features2` fix");
    let msg = "Fix `features2` usage\n\n\
        More information can be found at https://github.com/rust-lang/crates.io/issues/6135";
    repo.run_command(Command::new("git").args(["commit", "-am", msg]))?;

    let branch = match args.dry_run {
        false => "master",
        true => "features2-dry-run",
    };

    info!(?branch, "Pushing to upstream repository");
    repo.run_command(Command::new("git").args(["push", "origin", &format!("HEAD:{branch}")]))?;

    info!("`features2` fix completed");

    Ok(())
}

pub fn fix_features2(dry_run: bool) -> Job {
    Job::FixFeatures2(FixFeatures2Job { dry_run })
}
