use crate::{models::User, worker::Environment};
use crates_io_worker::BackgroundJob;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct UpdateFromGithub;

impl BackgroundJob for UpdateFromGithub {
    const JOB_NAME: &'static str = "update_from_github";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    /// Query the database for the next chunk of crates.io users to check. For each, query the
    /// GitHub API using the GitHub ID to see if their account has been deleted or renamed. Gather
    /// all the changes and update the `users` and `oauth_github` tables.
    async fn run(&self, _env: Self::Context) -> anyhow::Result<()> {
        let crates_io_users = next_user_batch().await?;

        let updates = refresh_users(crates_io_users).await?;

        apply_updates(updates).await?;

        Ok(())
    }
}

async fn next_user_batch() -> anyhow::Result<Vec<User>> {
    todo!();
}

#[derive(Debug, Clone)]
struct UsernameUpdate {
    user_id: i32,
    new_username: String,
}

async fn refresh_users(users: Vec<User>) -> anyhow::Result<Vec<UsernameUpdate>> {
    let mut updates = Vec::with_capacity(users.len());

    for user in users {
        if let Some(update) = refresh_user(user).await? {
            updates.push(update);
        }
    }

    Ok(updates)
}

async fn refresh_user(user: User) -> anyhow::Result<Option<UsernameUpdate>> {
    todo!();
}

async fn apply_updates(updates: Vec<UsernameUpdate>) -> anyhow::Result<()> {
    todo!();
}
