use crate::worker::Environment;
use crate::worker::jobs::ArchiveIndexBranch;
use anyhow::{Context, anyhow};
use chrono::Utc;
use crates_io_github::{CreateCommit, parse_github_slug};
use crates_io_worker::BackgroundJob;
use oauth2::AccessToken;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, instrument, warn};

const MASTER_REF: &str = "refs/heads/master";

async fn enqueue_archive_job(env: &Environment, branch: &str) -> anyhow::Result<()> {
    let conn = env.deadpool.get().await?;
    ArchiveIndexBranch::new(branch).enqueue(&conn).await?;
    Ok(())
}

/// Collapses the index into a single commit by driving the GitHub API
/// directly, without touching the local bare clone. This avoids the
/// pack generation that `git push` triggers for the full squash, which
/// has been OOMing the worker.
///
/// Git's object model has three layers we care about here: blobs hold
/// file contents, a tree maps names to blobs (and to nested trees) to
/// describe a directory snapshot, and a commit points at exactly one
/// top-level tree plus its parent commits and metadata. Crucially, a
/// tree is fully addressed by its SHA and is independent of any commit
/// that happens to reference it.
///
/// That independence is what makes this squash cheap: the current
/// `master` already points at a commit whose tree is exactly the state
/// we want to preserve, so we can build a brand-new parentless commit
/// that reuses that same tree SHA. No blobs or trees need to be
/// recomputed or uploaded; GitHub already has every object we
/// reference, and we just hand it a tiny new commit object and move
/// `master` to point at it.
#[derive(Serialize, Deserialize)]
pub struct SquashIndex;

impl BackgroundJob for SquashIndex {
    const JOB_NAME: &'static str = "squash_index";
    const DEDUPLICATED: bool = true;
    // Same queue as `SyncToGitIndex`, etc. so index-writing jobs serialize
    // against each other, even though this job does not touch the local
    // bare repo.
    const QUEUE: &'static str = "repository";

    type Context = Arc<Environment>;

    #[instrument(skip_all)]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        info!("Squashing the index into a single commit via the GitHub API");

        let github_app = env
            .github_app
            .as_ref()
            .ok_or_else(|| anyhow!("GitHub App is not configured"))?;

        let (owner, repo) = parse_github_slug(&env.repository_config.index_location)
            .context("Failed to parse index URL as `owner/repo`")?;

        let github = env.github.as_ref();

        let original_head = github.get_ref(&owner, &repo, MASTER_REF).await?;
        let original_sha = original_head.object.sha;
        info!("Read original HEAD: {original_sha}");

        let original_commit = github.get_commit(&owner, &repo, &original_sha).await?;
        let tree_sha = original_commit.tree.sha;

        let snapshot_branch = snapshot_branch_name();
        let message = squash_commit_message(&original_sha, &snapshot_branch);

        let token = github_app.installation_token().await?;
        let auth = AccessToken::new(token.expose_secret().into());

        let squash_start = Instant::now();
        let input = CreateCommit {
            message: &message,
            tree: &tree_sha,
            parents: &[],
        };
        let new_commit = github.create_commit(&owner, &repo, &input, &auth).await?;
        let new_sha = new_commit.sha;
        let duration = squash_start.elapsed().as_nanos();
        info!(duration, "Squash commit created: {new_sha}");

        // Create the snapshot ref first so that if anything after this
        // fails, `master` is still unmoved and the snapshot ref is
        // harmless (it points at the same SHA as `master`).
        let snapshot_ref = format!("refs/heads/{snapshot_branch}");
        github
            .create_ref(&owner, &repo, &snapshot_ref, &original_sha, &auth)
            .await?;

        // Best-effort drift check. GitHub has no CAS for refs, so the
        // `repository` queue is the real primary defense; this only
        // shrinks the race window.
        let current_head = github.get_ref(&owner, &repo, MASTER_REF).await?;
        if current_head.object.sha != original_sha {
            return Err(anyhow!(
                "`{}` drifted during squash (was {original_sha}, now {})",
                MASTER_REF,
                current_head.object.sha
            ));
        }

        github
            .update_ref(&owner, &repo, MASTER_REF, &new_sha, true, &auth)
            .await?;

        info!("The index has been successfully squashed.");

        if let Err(error) = enqueue_archive_job(&env, &snapshot_branch).await {
            warn!("Failed to enqueue `ArchiveIndexBranch` job for `{snapshot_branch}`: {error}");
        }

        Ok(())
    }
}

fn snapshot_branch_name() -> String {
    let now = Utc::now().format("%F");
    format!("snapshot-{now}")
}

fn squash_commit_message(original_head: impl fmt::Display, snapshot_branch: &str) -> String {
    format!(
        "Collapse index into one commit\n\n\
        Previous HEAD was {original_head}, now on the `{snapshot_branch}` branch\n\n\
        More information about this change can be found [online] and on [this issue].\n\n\
        [online]: https://internals.rust-lang.org/t/cargos-crate-index-upcoming-squash-into-one-commit/8440\n\
        [this issue]: https://github.com/rust-lang/crates-io-cargo-teams/issues/47"
    )
}
