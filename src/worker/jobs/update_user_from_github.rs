use crate::{
    models::OauthGithub,
    schema::{oauth_github, users},
    worker::Environment,
};
use chrono::Utc;
use crates_io_github::{GitHubError, GitHubUser};
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Serialize, Deserialize)]
pub struct UpdateUserFromGithub {
    /// Dry run will fetch updates from GitHub and log what it would change, but does not actually
    /// update the database.
    dry_run: bool,
    /// Crates.io user ID
    user_id: i32,
    /// GitHub ID
    account_id: i64,
    /// Encrypted GitHub token
    encrypted_token: Vec<u8>,
    /// Username currently in the database
    old_username: String,
}

impl BackgroundJob for UpdateUserFromGithub {
    const JOB_NAME: &'static str = "update_user_from_github";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    /// For the specified user, query the GitHub API for the user's current information to see if
    /// their account has been deleted or renamed. Update the `users` and `oauth_github` tables,
    /// saving the current time in `last_sync` even if the user information hasn't changed.
    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        info!(
            dry_run = self.dry_run,
            user_id = self.user_id,
            github_id = self.account_id,
            old_username = self.old_username,
            "Starting UpdateUserFromGithub"
        );

        let mut conn = ctx.deadpool.get().await?;

        let github_user = self.refresh_user(&ctx).await?;

        if self.dry_run {
            info!(
                user_id = self.user_id,
                old_username = self.old_username,
                new_username = github_user.login,
                "Dry run UpdateUserFromGithub proposed update"
            );
        } else {
            self.apply_update(&github_user, &mut conn).await;
        }

        Ok(())
    }
}

impl UpdateUserFromGithub {
    pub fn new(dry_run: bool, oauth_github: OauthGithub) -> Self {
        let OauthGithub {
            user_id,
            account_id,
            encrypted_token,
            login,
            ..
        } = oauth_github;

        Self {
            dry_run,
            user_id,
            account_id,
            encrypted_token,
            old_username: login,
        }
    }

    /// Given the current environment's context, request information from GitHub using the user's
    /// API token.
    async fn refresh_user(&self, ctx: &Arc<Environment>) -> anyhow::Result<GitHubUser> {
        // if the user's gh_id isn't positive, we don't even need to ask github about this,
        // we know this user is invalid. Just make sure their username is the ghost username.
        if self.account_id < 1 {
            Ok(self.ghost_user())
        } else {
            let github = ctx.github.as_ref();
            let token = ctx
                .config
                .gh_token_encryption
                .decrypt(&self.encrypted_token)?;

            match github.current_user(&token).await {
                Ok(github_user) => Ok(github_user),
                // If the user is not found, the account has been deleted. Update to the ghost
                // username.
                Err(GitHubError::NotFound(_)) => Ok(self.ghost_user()),
                // Unauthorized/forbidden could mean:
                //
                // - the token we have for this user is out-of-date
                // - the user has revoked crates.io's oauth access
                //
                // In those cases, try to request the user's info via an unauthenticated GitHub
                // API request, unless they are a GitHub Enterprise Managed User as indicated by an
                // underscore in their username because we have to be authorized by the managing
                // enterprise to see any information on enterprise managed users.
                Err(GitHubError::Unauthorized(_)) | Err(GitHubError::Forbidden(_)) => {
                    // Enterprise managed users are the only ones that should contain underscores.
                    if self.old_username.contains('_') {
                        // We can't get updated info, so keep what we have.
                        Ok(GitHubUser {
                            login: self.old_username.clone(),
                            id: self.account_id as i32,
                            // The other fields are not used in `apply_update`.
                            avatar_url: Default::default(),
                            email: Default::default(),
                            name: Default::default(),
                        })
                    } else {
                        match github.get_user_by_id(self.account_id).await {
                            Ok(github_user) => Ok(github_user),
                            Err(GitHubError::NotFound(_)) => Ok(self.ghost_user()),
                            // Not sure how we could get Unauthorized/Forbidden for an anonymous
                            // API request. We could get rate limited though; if that's the case,
                            // stop and try this user again later.
                            Err(e) => Err(e.into()),
                        }
                    }
                }
                // If we get another sort of error, it may be transient; stop and try this user
                // again later.
                Err(e @ GitHubError::Other(_)) => Err(e.into()),
            }
        }
    }

    /// Given the information from GitHub about the current user, make the appropriate changes to
    /// the `users` and `oauth_github` tables.
    async fn apply_update(&self, github_user: &GitHubUser, conn: &mut AsyncPgConnection) {
        // Use a transaction so that we either update both or neither the `users` record and the
        // corresponding `oauth_github` record. If neither are updated, log and continue to the
        // next user rather than stopping-- hopefully we'll get that user updated next time.
        if let Err(e) = conn
            .transaction(async |conn| {
                // This will be removed when we no longer sync crates.io usernames with GitHub.
                // (The transaction can be removed when this is removed as well)
                // It's only needed if there's a change in username.
                if self.old_username != github_user.login {
                    diesel::update(users::table)
                        .filter(users::id.eq(self.user_id))
                        .set(users::gh_login.eq(&github_user.login))
                        .execute(conn)
                        .await?;
                }

                // This update is needed even if there's no change in username to set the
                // `last_sync` time to `now`.
                diesel::update(oauth_github::table)
                    .filter(oauth_github::user_id.eq(self.user_id))
                    .set((
                        oauth_github::login.eq(&github_user.login),
                        oauth_github::last_sync.eq(Utc::now()),
                    ))
                    .execute(conn)
                    .await?;

                Ok::<(), diesel::result::Error>(())
            })
            .await
        {
            // Database update failed; it's ok to not update this user this round.
            // Better luck next time.
            error!(
                "Could not update user ID {} from username {} to username {}: {e}",
                self.user_id, self.old_username, github_user.login,
            );
        }
    }

    /// If this user has been deleted, ensure their username has been changed to
    /// `ghost_{crates.io id}` to ensure uniqueness by creating a `GitHubUser` by hand.
    fn ghost_user(&self) -> GitHubUser {
        GitHubUser {
            avatar_url: None,
            email: None,
            id: self.account_id as i32,
            login: format!("ghost_{}", self.user_id),
            name: None,
        }
    }
}
