use crate::{models::User, schema::*, worker::Environment};
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
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
    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let mut conn = ctx.deadpool.get().await?;

        let metadata = get_state_params(&mut conn).await?;

        let crates_io_users = next_user_batch(metadata, &mut conn).await?;

        let updates = refresh_users(crates_io_users).await?;

        apply_updates(updates).await?;

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct MetadataGithubRefresh {
    highest_processed_user_id: i32,
    batch_size: i64,
}

/// Query the metadata stored in the database from previous runs of this job to know which users
/// are in the next batch.
async fn get_state_params(conn: &mut AsyncPgConnection) -> anyhow::Result<MetadataGithubRefresh> {
    let (highest_processed_user_id, batch_size) = metadata_github_refresh::table
        .select((
            metadata_github_refresh::highest_processed_user_id,
            metadata_github_refresh::batch_size,
        ))
        .first(conn)
        .await?;

    Ok(MetadataGithubRefresh {
        highest_processed_user_id,
        batch_size,
    })
}

async fn next_user_batch(
    MetadataGithubRefresh {
        highest_processed_user_id,
        batch_size,
    }: MetadataGithubRefresh,
    conn: &mut AsyncPgConnection,
) -> anyhow::Result<Vec<User>> {
    Ok(User::query()
        .filter(users::id.gt(highest_processed_user_id))
        .order(users::id.asc())
        .limit(batch_size)
        .load(conn)
        .await?)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crates_io_test_db::TestDatabase;

    async fn new_user(username: &str, conn: &mut AsyncPgConnection) -> User {
        use crate::models::NewUser;

        NewUser::builder()
            .gh_id(0)
            .gh_login(username)
            .gh_encrypted_token(&[])
            .build()
            .insert(conn)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn load_params_from_metadata_table() {
        let db = TestDatabase::new();
        let mut conn = db.async_connect().await;

        let metadata = get_state_params(&mut conn).await.unwrap();
        assert_eq!(
            metadata,
            MetadataGithubRefresh {
                highest_processed_user_id: 0,
                batch_size: 100,
            }
        );
    }

    async fn user_batch(
        highest_processed_user_id: i32,
        batch_size: i64,
        conn: &mut AsyncPgConnection,
    ) -> Vec<User> {
        next_user_batch(
            MetadataGithubRefresh {
                highest_processed_user_id,
                batch_size,
            },
            conn,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn get_batches_as_specified() {
        let db = TestDatabase::new();
        let mut conn = db.async_connect().await;

        let u1 = new_user("foo", &mut conn).await;
        let u2 = new_user("baz", &mut conn).await;
        let u3 = new_user("bar", &mut conn).await;
        let users = vec![u1, u2, u3];

        let all = user_batch(0, 100, &mut conn).await;
        assert_eq!(&all[..], &users[..]);

        let limit_num = user_batch(0, 2, &mut conn).await;
        assert_eq!(&limit_num[..], &users[0..=1]);

        let change_start = user_batch(1, 100, &mut conn).await;
        assert_eq!(&change_start[..], &users[1..=2]);

        let limit_num_and_change_start = user_batch(1, 1, &mut conn).await;
        assert_eq!(&limit_num_and_change_start[..], &users[1..=1]);
    }
}
