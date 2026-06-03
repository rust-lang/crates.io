use crate::{
    models::OauthGithub,
    schema::{oauth_github, users},
    worker::Environment,
};
use anyhow::anyhow;
use chrono::Utc;
use crates_io_github::{GitHubError, GitHubUser};
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use oauth2::AccessToken;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Serialize, Deserialize)]
pub struct UpdateUserFromGithub {
    /// Dry run will fetch updates from GitHub and log what it would change, but does not actually
    /// update the database.
    pub dry_run: bool,
    /// GitHub ID
    pub account_id: i64,
}

impl BackgroundJob for UpdateUserFromGithub {
    const JOB_NAME: &'static str = "update_user_from_github";
    const DEDUPLICATED: bool = true;
    // These jobs aren't urgent and shouldn't page anyone if they take a long time.
    const PRIORITY: i16 = -15;

    type Context = Arc<Environment>;

    /// For the specified user, query the GitHub API for the user's current information to see if
    /// their account has been deleted or renamed. Update the `users` and `oauth_github` tables,
    /// saving the current time in `last_sync` even if the user information hasn't changed.
    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let mut conn = ctx.deadpool.get().await?;

        // If no oauth_github info with this account id is found, then the record has been deleted
        // since this job was enqueued. Stop and exit with success so we don't retry the job.
        let oauth_github = oauth_github::table
            .filter(oauth_github::account_id.eq(self.account_id))
            .first::<OauthGithub>(&mut conn)
            .await?;

        info!(
            "Starting UpdateUserFromGithub ({}): user_id {}, github_id {}, old username {}",
            if self.dry_run { "DRY RUN" } else { "FOR REAL" },
            oauth_github.user_id,
            self.account_id,
            oauth_github.login,
        );

        let github_user = self.refresh_user(&ctx, &oauth_github).await?;

        if self.dry_run {
            info!(
                "Dry run UpdateUserFromGithub proposed update for crates.io user {} \
                from username `{}` to username `{}`",
                oauth_github.user_id, oauth_github.login, github_user.login,
            );
        } else {
            self.apply_update(&oauth_github, &github_user, &mut conn)
                .await;
        }

        Ok(())
    }
}

impl UpdateUserFromGithub {
    /// Given the current environment's context, request information from GitHub using the user's
    /// API token.
    async fn refresh_user(
        &self,
        ctx: &Arc<Environment>,
        oauth_github: &OauthGithub,
    ) -> anyhow::Result<GitHubUser> {
        let github = ctx.github.as_ref();
        let token = ctx
            .config
            .gh_token_encryption
            .decrypt(&oauth_github.encrypted_token)?;

        match github.current_user(&token).await {
            Ok(github_user) => Ok(github_user),
            // If the user is not found, the account has been deleted. Update to the ghost
            // username.
            Err(GitHubError::NotFound(_)) => Ok(self.ghost_user(oauth_github.user_id)),
            // Unauthorized/forbidden could mean:
            //
            // - the token we have for this user is out-of-date
            // - the user has revoked crates.io's oauth access
            //
            // In those cases, try to request the user's info via a GitHub API request
            // authenticated with our sync GitHub app's token, unless they are a GitHub Enterprise
            // indicated by an underscore in their username because we have to be authorized by the
            // managing enterprise to see any information on enterprise managed users.
            Err(GitHubError::Unauthorized(_)) | Err(GitHubError::Forbidden(_)) => {
                // Enterprise managed users are the only ones that should contain underscores.
                if oauth_github.login.contains('_') {
                    // We can't get updated info, so keep what we have.
                    Ok(GitHubUser {
                        login: oauth_github.login.clone(),
                        id: self.account_id as i32,
                        // The other fields are not used in `apply_update`.
                        avatar_url: Default::default(),
                        email: Default::default(),
                        name: Default::default(),
                    })
                } else {
                    let Some(sync_github_app) = ctx.sync_github_app.as_ref() else {
                        let error =
                            anyhow!("sync github app not configured, can't make user API request");
                        return Err(error);
                    };

                    let token = AccessToken::new(
                        sync_github_app
                            .installation_token()
                            .await?
                            .expose_secret()
                            .into(),
                    );

                    match github.get_user_by_id(self.account_id, &token).await {
                        Ok(github_user) => Ok(github_user),
                        Err(GitHubError::NotFound(_)) => Ok(self.ghost_user(oauth_github.user_id)),
                        // For any other error, stop and try this user again later.
                        Err(e) => Err(e.into()),
                    }
                }
            }
            // If we get another sort of error, it may be transient; stop and try this user
            // again later.
            Err(e @ GitHubError::Other(_)) => Err(e.into()),
        }
    }

    /// Given the information from GitHub about the current user, make the appropriate changes to
    /// the `users` and `oauth_github` tables.
    async fn apply_update(
        &self,
        oauth_github: &OauthGithub,
        github_user: &GitHubUser,
        conn: &mut AsyncPgConnection,
    ) {
        // Use a transaction so that we either update both or neither the `users` record and the
        // corresponding `oauth_github` record. If neither are updated, log and continue to the
        // next user rather than stopping-- hopefully we'll get that user updated next time.
        if let Err(e) = conn
            .transaction(async |conn| {
                // This will be removed when we no longer sync crates.io usernames with GitHub.
                // (The transaction can be removed when this is removed as well)
                // It's only needed if there's a change in username.
                if oauth_github.login != github_user.login {
                    diesel::update(users::table)
                        .filter(users::id.eq(oauth_github.user_id))
                        .set(users::gh_login.eq(&github_user.login))
                        .execute(conn)
                        .await?;
                }

                // This update is needed even if there's no change in username to set the
                // `last_sync` time to `now`.
                diesel::update(oauth_github::table)
                    .filter(oauth_github::account_id.eq(self.account_id))
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
                oauth_github.user_id, oauth_github.login, github_user.login,
            );
        }
    }

    /// If this user has been deleted, ensure their username has been changed to
    /// `ghost_{crates.io id}` to ensure uniqueness by creating a `GitHubUser` by hand.
    fn ghost_user(&self, user_id: i32) -> GitHubUser {
        GitHubUser {
            avatar_url: None,
            email: None,
            id: self.account_id as i32,
            login: format!("ghost_{}", user_id),
            name: None,
        }
    }
}
