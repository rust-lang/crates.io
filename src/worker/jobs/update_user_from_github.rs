use crate::worker::Environment;
use crates_io_github::GitHubUser;
use crates_io_worker::BackgroundJob;
use diesel_async::AsyncPgConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct UpdateUserFromGithub {
    /// Crates.io user ID
    user_id: i32,
    /// GitHub ID
    account_id: i32,
    /// Encrypted GitHub token
    encrypted_token: Vec<u8>,
}

impl BackgroundJob for UpdateUserFromGithub {
    const JOB_NAME: &'static str = "update_user_from_github";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    /// For the specified user, query the GitHub API for the user's current information to see if
    /// their account has been deleted or renamed. Update the `users` and `oauth_github` tables,
    /// saving the current time in `last_sync` even if the user information hasn't changed.
    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let mut conn = ctx.deadpool.get().await?;

        let github_user = self.refresh_user(&ctx).await?;

        self.apply_update(&github_user, &mut conn).await;

        Ok(())
    }
}

impl UpdateUserFromGithub {
    /// Given the current environment's context, request information from GitHub using the user's
    /// API token.
    async fn refresh_user(&self, _ctx: &Arc<Environment>) -> anyhow::Result<GitHubUser> {
        todo!();
    }

    /// Given the information from GitHub about the current user, make the appropriate changes to
    /// the `users` and `oauth_github` tables.
    async fn apply_update(&self, _github_user: &GitHubUser, _conn: &mut AsyncPgConnection) {
        todo!();
    }
}
