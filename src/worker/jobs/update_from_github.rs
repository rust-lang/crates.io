use crate::{models::User, schema::*, worker::Environment};
use crates_io_github::{GitHubClient, GitHubError, RealGitHubClient};
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;

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
        let user_agent = crates_io_version::user_agent();
        let client = Client::builder().user_agent(user_agent).build()?;
        let github = RealGitHubClient::new(client);

        let metadata = get_state_params(&mut conn).await?;

        let crates_io_users = next_user_batch(metadata, &mut conn).await?;

        let updates = refresh_users(
            &github,
            &ctx.config.gh_client_id,
            ctx.config.gh_client_secret.secret(),
            crates_io_users,
        )
        .await?;

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

#[derive(Debug, Clone, PartialEq)]
struct UsernameUpdate {
    id: i32,
    new_username: String,
}

async fn refresh_users(
    github: &dyn GitHubClient,
    username: &str,
    password: &str,
    users: Vec<User>,
) -> anyhow::Result<Vec<UsernameUpdate>> {
    let mut updates = Vec::with_capacity(users.len());

    for user in users {
        if let Some(update) = refresh_user(github, username, password, user).await? {
            updates.push(update);
        }
    }

    Ok(updates)
}

async fn refresh_user(
    github: &dyn GitHubClient,
    username: &str,
    password: &str,
    user: User,
) -> anyhow::Result<Option<UsernameUpdate>> {
    // Can't check github if we don't have a real github id!
    if user.gh_id < 1 {
        // Make sure users that don't have any association to github anymore have a
        // crates.io username that's guaranteed to be unique by using crates.io's ID.
        let ghost_username = format!("ghost_{}", user.id);
        if user.gh_login != ghost_username {
            Ok(Some(UsernameUpdate {
                id: user.id,
                new_username: ghost_username,
            }))
        } else {
            // User already has their crates.io username set to `ghost_{crates.io ID}`
            Ok(None)
        }
    } else {
        // Wait a second between GitHub API requests to stay under the rate limit.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        match github
            .get_user_by_id(user.gh_id as i64, username, password)
            .await
        {
            Ok(github_user) => {
                if user.gh_login != github_user.login {
                    // User renamed their github account
                    Ok(Some(UsernameUpdate {
                        id: user.id,
                        new_username: github_user.login,
                    }))
                } else {
                    // The github username we have is current
                    Ok(None)
                }
            }
            Err(GitHubError::NotFound(..)) => {
                // User deleted their github account
                let ghost_username = format!("ghost_{}", user.id);
                Ok(Some(UsernameUpdate {
                    id: user.id,
                    new_username: ghost_username,
                }))
            }
            Err(e) => {
                error!(
                    username = user.gh_login,
                    user_id = user.id,
                    user_github_id = user.gh_id,
                    "Could not check GitHub info: {e}"
                );
                // GitHub request couldn't finish; it's ok to not update this user this round.
                // Better luck next time.
                Ok(None)
            }
        }
    }
}

async fn apply_updates(updates: Vec<UsernameUpdate>) -> anyhow::Result<()> {
    todo!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crates_io_github::{GitHubUser, MockGitHubClient};
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

    fn user(id: i32, gh_id: i32, gh_login: &str) -> User {
        User {
            id,
            gh_id,
            gh_login: gh_login.to_string(),
            account_lock_reason: None,
            account_lock_until: None,
            gh_avatar: None,
            gh_encrypted_token: Default::default(),
            is_admin: false,
            name: None,
            publish_notifications: Default::default(),
        }
    }

    fn github_user(id: i32, login: &str) -> GitHubUser {
        GitHubUser {
            id,
            login: login.to_string(),
            avatar_url: None,
            email: None,
            name: None,
        }
    }

    #[tokio::test]
    async fn refresh_gets_expected_updates() {
        use mockall::predicate::*;

        let users = vec![
            user(1, 100, "no_updates_needed"),
            user(2, -1, "negative_1_github_id"),
            user(3, 0, "zero_github_id"),
            user(4, -9000, "ghost_4"),
            user(5, 105, "old_github_username"),
            user(6, 106, "deleted_github"),
            user(7, 107, "github_is_down"),
        ];

        let mut github_mock = MockGitHubClient::new();
        github_mock
            .expect_get_user_by_id()
            .with(eq(100), always(), always())
            .returning(|_, _, _| Ok(github_user(100, "no_updates_needed")));
        github_mock
            .expect_get_user_by_id()
            .with(eq(105), always(), always())
            .returning(|_, _, _| Ok(github_user(105, "new_github_username")));
        github_mock
            .expect_get_user_by_id()
            .with(eq(106), always(), always())
            .returning(|_, _, _| Err(GitHubError::NotFound(anyhow::anyhow!("No user here"))));
        github_mock
            .expect_get_user_by_id()
            .with(eq(107), always(), always())
            .returning(|_, _, _| Err(GitHubError::Other(anyhow::anyhow!("Unicorn!!"))));

        let updates = refresh_users(&github_mock, "foo", "bar", users)
            .await
            .unwrap();
        assert_eq!(
            updates,
            vec![
                // User 1 doesn't need an update because their crates.io username matches GitHub's.
                UsernameUpdate {
                    id: 2,
                    new_username: "ghost_2".into(),
                },
                UsernameUpdate {
                    id: 3,
                    new_username: "ghost_3".into(),
                },
                // User 4 doesn't need an update because their username is already `ghost_{id}`.
                UsernameUpdate {
                    id: 5,
                    new_username: "new_github_username".into(),
                },
                UsernameUpdate {
                    id: 6,
                    new_username: "ghost_6".into(),
                } // User 7 doesn't need an update because GitHub was down when we requested it
            ]
        );
    }
}
