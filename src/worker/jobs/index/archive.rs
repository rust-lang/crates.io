use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use anyhow::{Context, anyhow};
use crates_io_github::parse_github_slug;
use crates_io_worker::BackgroundJob;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Command;
use tracing::{info, instrument, warn};
use url::Url;

const REMOTE_NAME: &str = "archive";
const TEMP_BRANCH: &str = "temp";
const COMMITS_PER_PUSH: usize = 100_000;

/// A contiguous commit range ending at the checkpoint pushed to the archive.
#[derive(Debug, PartialEq)]
struct ArchivePushChunk {
    start: usize,
    end: usize,
    commit: String,
}

/// Splits an oldest-to-newest commit list into archive push checkpoints.
fn archive_push_chunks(commits: &str, chunk_size: usize) -> Vec<ArchivePushChunk> {
    let commits = commits.lines().collect::<Vec<_>>();
    commits
        .chunks(chunk_size)
        .enumerate()
        .map(|(index, chunk)| {
            let start = index * chunk_size;
            ArchivePushChunk {
                start,
                end: start + chunk.len(),
                commit: chunk.last().unwrap().to_string(),
            }
        })
        .collect()
}

#[derive(Serialize, Deserialize)]
pub struct ArchiveIndexBranch {
    branch: String,
}

impl ArchiveIndexBranch {
    pub fn new(branch: impl Into<String>) -> Self {
        Self {
            branch: branch.into(),
        }
    }
}

impl BackgroundJob for ArchiveIndexBranch {
    const JOB_NAME: &'static str = "archive_index_branch";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    /// Mirrors a snapshot branch from the crate index to the configured archive
    /// repository. No-op when no archive URL is configured.
    ///
    /// Each invocation works against a fresh, ephemeral bare clone of the
    /// snapshot branch in a `TempDir`. The job does not share state with the
    /// long-lived bare clone behind `Environment::lock_index()`.
    #[instrument(skip_all, fields(branch = self.branch))]
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        let Some(archive_url) = env.config.index_archive_url.as_ref() else {
            info!("`index_archive_url` not configured, skipping archive push");
            return Ok(());
        };

        let Some(index_sync_github_app) = env.index_sync_github_app.as_ref() else {
            let error =
                anyhow!("`index_archive_url` is set but index sync GitHub App is not configured");
            return Err(error);
        };

        let clone_url = clone_url(&env.repository_config.index_location)?;
        info!(
            "Cloning snapshot branch ({branch}) from the index repository ({clone_url})",
            branch = self.branch
        );

        // `TempDir` create/drop are sync filesystem I/O. The bare clone is one
        // large packfile plus a handful of small refs/config files, so
        // `remove_dir_all` cost is bounded by inode count, not pack size, and
        // should stay brief enough to run on the async runtime.
        let tempdir = tempfile::Builder::new()
            .prefix("snapshot-clone")
            .tempdir()?;

        let clone_start = Instant::now();
        let output = Command::new("git")
            .args([
                "clone",
                "--bare",
                "--single-branch",
                "--branch",
                &self.branch,
                clone_url.as_str(),
            ])
            // `tempdir.path()` is `&Path`, so it can't share the `&str` array above.
            .arg(tempdir.path())
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!(
                "git clone failed: {}{}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            ));
        }

        info!(
            duration = clone_start.elapsed().as_nanos(),
            "Cloned snapshot branch ({branch})",
            branch = self.branch,
        );

        let token = index_sync_github_app.installation_token().await?;
        let push_url = match build_credentialed_url(archive_url, token.expose_secret()) {
            Ok(url) => url,
            Err(()) => {
                warn!(
                    "Archive URL ({archive_url}) does not support credentials; pushing without auth"
                );
                archive_url.clone()
            }
        };

        let bare_path = tempdir.path().to_owned();
        spawn_blocking(move || -> anyhow::Result<()> {
            // Use `git2` here so the credentialed URL is written only into the
            // tempdir's `.git/config` and never appears in process argv or logs.
            let repo = git2::Repository::open_bare(&bare_path)?;
            repo.remote(REMOTE_NAME, push_url.as_str())?;
            Ok(())
        })
        .await??;
        info!("Added archive repository as `{REMOTE_NAME}` remote");

        let output = Command::new("git")
            .current_dir(tempdir.path())
            .args(["rev-list", "--reverse", &self.branch])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!(
                "git rev-list failed: {}{}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            ));
        }

        let commits =
            String::from_utf8(output.stdout).context("git rev-list output was not UTF-8")?;
        let chunks = archive_push_chunks(&commits, COMMITS_PER_PUSH);
        let commit_count = chunks.last().map_or(0, |chunk| chunk.end);
        let chunk_count = chunks.len();
        info!(
            "Found {commit_count} commits in {chunk_count} chunks for snapshot branch ({branch})",
            branch = self.branch,
        );

        for chunk in chunks {
            let ArchivePushChunk { start, end, commit } = chunk;
            info!(
                "Pushing commits {start} to {end} for snapshot branch ({branch}) at commit {commit}",
                branch = self.branch,
            );

            let refspec = format!("+{commit}:refs/heads/{TEMP_BRANCH}");
            let push_start = Instant::now();
            let output = Command::new("git")
                .current_dir(tempdir.path())
                .args(["push", REMOTE_NAME, &refspec])
                .output()
                .await
                .with_context(|| {
                    format!(
                        "Failed to run git push for commits {start} to {end} of snapshot branch ({}) at commit {commit}",
                        self.branch
                    )
                })?;

            if !output.status.success() {
                return Err(anyhow!(
                    "git push failed for commits {start} to {end} of snapshot branch ({}) at commit {commit}: {}{}",
                    self.branch,
                    String::from_utf8_lossy(&output.stderr),
                    String::from_utf8_lossy(&output.stdout)
                ));
            }

            info!(
                duration = push_start.elapsed().as_nanos(),
                "Pushed commits {start} to {end} for snapshot branch ({branch}) at commit {commit}",
                branch = self.branch,
            );
        }

        info!(
            "Pushing snapshot branch ({branch}) to archive repository ({archive_url})",
            branch = self.branch
        );
        let refspec = format!("{branch}:refs/heads/{branch}", branch = self.branch);
        let push_start = Instant::now();
        let output = Command::new("git")
            .current_dir(tempdir.path())
            .args(["push", REMOTE_NAME, &refspec])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!(
                "git push failed: {}{}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            ));
        }

        info!(
            duration = push_start.elapsed().as_nanos(),
            "Pushed snapshot branch ({branch}) to archive repository",
            branch = self.branch,
        );

        info!("Deleting temporary archive branch (`{TEMP_BRANCH}`)");
        let delete_start = Instant::now();
        let output = Command::new("git")
            .current_dir(tempdir.path())
            .args(["push", REMOTE_NAME, "--delete", TEMP_BRANCH])
            .output()
            .await;

        match output {
            Ok(output) if output.status.success() => {
                info!(
                    duration = delete_start.elapsed().as_nanos(),
                    "Deleted temporary archive branch (`{TEMP_BRANCH}`)"
                );
            }
            Ok(output) => {
                warn!(
                    "Failed to delete temporary archive branch (`{TEMP_BRANCH}`): {}{}",
                    String::from_utf8_lossy(&output.stderr),
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            Err(error) => {
                warn!("Failed to delete temporary archive branch (`{TEMP_BRANCH}`): {error}");
            }
        }

        info!("Archived snapshot branch ({branch})", branch = self.branch,);

        Ok(())
    }
}

/// Returns a copy of `base` with `x-access-token` / `token` embedded as the
/// HTTPS credentials git consumes when pushing. Returns `Err(())` when the
/// URL scheme does not allow userinfo (e.g. `file://`).
fn build_credentialed_url(base: &Url, token: &str) -> Result<Url, ()> {
    let mut url = base.clone();
    url.set_username("x-access-token")?;
    url.set_password(Some(token))?;
    Ok(url)
}

/// Picks the URL to use for the read-only clone of the index.
///
/// `GIT_REPO_URL` is configured as an SSH URL in production so the index
/// workers that push can authenticate with the deploy key. github.com
/// rejects unauthenticated SSH even for public repositories, and this job
/// only needs to read, so rewrite github URLs to HTTPS. Non-github URLs
/// (e.g. `file://` in tests) are returned unchanged.
fn clone_url(configured: &Url) -> anyhow::Result<Url> {
    if configured.host_str() != Some("github.com") {
        return Ok(configured.clone());
    }

    let (owner, repo) =
        parse_github_slug(configured).context("Failed to parse index URL as `owner/repo`")?;

    format!("https://github.com/{owner}/{repo}.git")
        .parse()
        .context("Failed to build HTTPS clone URL")
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_err;
    use insta::assert_snapshot;

    fn chunk(start: usize, end: usize, commit: &str) -> ArchivePushChunk {
        ArchivePushChunk {
            start,
            end,
            commit: commit.into(),
        }
    }

    #[test]
    fn archive_push_chunks_include_partial_final_chunk() {
        let chunks = archive_push_chunks("a\nb\nc\nd\ne\n", 2);

        assert_eq!(
            chunks,
            [chunk(0, 2, "b"), chunk(2, 4, "d"), chunk(4, 5, "e")]
        );
    }

    #[test]
    fn archive_push_chunks_do_not_duplicate_exact_final_chunk() {
        let chunks = archive_push_chunks("a\nb\nc\nd\n", 2);

        assert_eq!(chunks, [chunk(0, 2, "b"), chunk(2, 4, "d")]);
    }

    #[test]
    fn build_credentialed_url_https() {
        let url: Url = "https://github.com/rust-lang/archive.git".parse().unwrap();
        let credentialed = build_credentialed_url(&url, "tok").unwrap();
        assert_snapshot!(credentialed, @"https://x-access-token:tok@github.com/rust-lang/archive.git");
    }

    #[test]
    fn build_credentialed_url_file_rejected() {
        let url: Url = "file:///tmp/archive".parse().unwrap();
        assert_err!(build_credentialed_url(&url, "tok"));
    }

    #[test]
    fn clone_url_rewrites_github_ssh_to_https() {
        let configured: Url = "ssh://git@github.com/rust-lang/crates.io-index.git"
            .parse()
            .unwrap();
        let rewritten = clone_url(&configured).unwrap();
        assert_snapshot!(rewritten, @"https://github.com/rust-lang/crates.io-index.git");
    }

    #[test]
    fn clone_url_normalizes_github_https() {
        let configured: Url = "https://github.com/rust-lang/crates.io-index"
            .parse()
            .unwrap();
        let rewritten = clone_url(&configured).unwrap();
        assert_snapshot!(rewritten, @"https://github.com/rust-lang/crates.io-index.git");
    }

    #[test]
    fn clone_url_preserves_non_github() {
        let configured: Url = "file:///tmp/upstream".parse().unwrap();
        let rewritten = clone_url(&configured).unwrap();
        assert_snapshot!(rewritten, @"file:///tmp/upstream");
    }

    #[test]
    fn clone_url_rejects_malformed_github() {
        let configured: Url = "https://github.com/rust-lang".parse().unwrap();
        assert_err!(clone_url(&configured));
    }
}
