use crate::worker::WorkerContext;
use anyhow::{Context, anyhow};
use crates_io_github::{GitHubAuth, parse_github_slug};
use crates_io_worker::BackgroundJob;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

/// Deletes an archived snapshot branch from the source index repository.
#[derive(Serialize, Deserialize)]
pub struct DeleteArchivedIndexBranch {
    branch: String,
}

impl DeleteArchivedIndexBranch {
    /// Creates a cleanup job for the given snapshot branch.
    pub fn new(branch: impl Into<String>) -> Self {
        let branch = branch.into();
        Self { branch }
    }
}

impl BackgroundJob for DeleteArchivedIndexBranch {
    const JOB_NAME: &'static str = "delete_archived_index_branch";
    const DEDUPLICATED: bool = true;

    type Context = WorkerContext;

    /// Removes a snapshot branch from the source index repository.
    #[instrument(skip_all, fields(branch = self.branch))]
    async fn run(self, ctx: Self::Context) -> anyhow::Result<()> {
        let branch = &self.branch;
        if !branch.starts_with("snapshot-") {
            anyhow::bail!("Refusing to delete non-snapshot index branch `{branch}`");
        }

        let app = ctx
            .index_sync_github_app
            .as_ref()
            .ok_or_else(|| anyhow!("index sync GitHub App is not configured"))?;

        let (owner, repo) = parse_github_slug(&ctx.repository_config.index_location)
            .context("Failed to parse index URL as `owner/repo`")?;

        let token = app.installation_token().await?;

        let ref_name = format!("refs/heads/{branch}");
        let auth = GitHubAuth::bearer(token);
        let github = ctx.github.as_ref();
        github.delete_ref(&owner, &repo, &ref_name, &auth).await?;

        info!("Deleted snapshot branch `{branch}` from index repository");
        Ok(())
    }
}
