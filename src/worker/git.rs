use crate::background_jobs::Environment;
use crate::git::Crate;
use crate::schema;
use anyhow::Context;
use chrono::Utc;
use diesel::prelude::*;
use std::fs::{self, OpenOptions};
use std::process::Command;
use swirl::PerformError;

#[swirl::background_job]
pub fn add_crate(env: &Environment, krate: Crate) -> Result<(), PerformError> {
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

    Ok(())
}

/// Yanks or unyanks a crate version. This requires finding the index
/// file, deserlialise the crate from JSON, change the yank boolean to
/// `true` or `false`, write all the lines back out, and commit and
/// push the changes.
#[swirl::background_job]
pub fn sync_yanked(
    env: &Environment,
    conn: &PgConnection,
    krate: String,
    version_num: String,
) -> Result<(), PerformError> {
    trace!(?krate, ?version_num, "Load yanked status from database");

    let yanked: bool = schema::versions::table
        .inner_join(schema::crates::table)
        .filter(schema::crates::name.eq(&krate))
        .filter(schema::versions::num.eq(&version_num))
        .select(schema::versions::yanked)
        .get_result(conn)
        .context("Failed to load yanked status from database")?;

    trace!(?krate, ?version_num, yanked);

    let repo = env.lock_index()?;
    let dst = repo.index_file(&krate);

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

    Ok(())
}

/// Collapse the index into a single commit, archiving the current history in a snapshot branch.
#[swirl::background_job]
pub fn squash_index(env: &Environment) -> Result<(), PerformError> {
    let repo = env.lock_index()?;
    println!("Squashing the index into a single commit.");

    let now = Utc::now().format("%Y-%m-%d");
    let original_head = repo.head_oid()?.to_string();
    let msg = format!("Collapse index into one commit\n\n\
        Previous HEAD was {}, now on the `snapshot-{}` branch\n\n\
        More information about this change can be found [online] and on [this issue].\n\n\
        [online]: https://internals.rust-lang.org/t/cargos-crate-index-upcoming-squash-into-one-commit/8440\n\
        [this issue]: https://github.com/rust-lang/crates-io-cargo-teams/issues/47", original_head, now);

    repo.squash_to_single_commit(&msg)?;

    // Shell out to git because libgit2 does not currently support push leases

    repo.run_command(Command::new("git").args(&[
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

    println!("The index has been successfully squashed.");

    Ok(())
}
