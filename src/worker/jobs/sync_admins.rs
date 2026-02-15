use crate::email::EmailMessage;
use crate::schema::{emails, oauth_github, users};
use crate::worker::Environment;
use anyhow::Context;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use minijinja::context;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// See <https://github.com/rust-lang/team/pull/1197>.
const PERMISSION_NAME: &str = "crates_io_admin";

#[derive(Serialize, Deserialize)]
pub struct SyncAdmins;

impl BackgroundJob for SyncAdmins {
    const JOB_NAME: &'static str = "sync_admins";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        info!("Syncing admins from rust-lang/team repo…");

        let repo_admins = ctx.team_repo.get_permission(PERMISSION_NAME).await?.people;
        let repo_admin_ids = repo_admins
            .iter()
            .map(|m| m.github_id)
            .collect::<HashSet<_>>();

        let mut conn = ctx.deadpool.get().await?;

        let format_repo_admins = |github_ids: &HashSet<i64>| {
            repo_admins
                .iter()
                .filter(|m| github_ids.contains(&m.github_id))
                .map(|m| format!("{} (github_id: {})", m.github, m.github_id))
                .collect::<Vec<_>>()
        };

        // Existing admins from the database.

        let database_admins = (users::table.inner_join(oauth_github::table))
            .left_join(emails::table)
            .select((
                oauth_github::account_id,
                oauth_github::login,
                emails::email.nullable(),
            ))
            .filter(users::is_admin.eq(true))
            .get_results::<(i64, String, Option<String>)>(&mut conn)
            .await?;

        let database_admin_ids = database_admins
            .iter()
            .map(|(gh_id, _, _)| *gh_id)
            .collect::<HashSet<_>>();

        let format_database_admins = |github_ids: &HashSet<i64>| {
            database_admins
                .iter()
                .filter(|(gh_id, _, _)| github_ids.contains(gh_id))
                .map(|(gh_id, login, _)| format!("{login} (github_id: {gh_id})"))
                .collect::<Vec<_>>()
        };

        // New admins from the team repo that don't have admin access yet.

        let new_admin_ids = repo_admin_ids
            .difference(&database_admin_ids)
            .copied()
            .collect::<HashSet<_>>();

        let added_admin_ids = if new_admin_ids.is_empty() {
            Vec::new()
        } else {
            let new_admins = format_repo_admins(&new_admin_ids).join(", ");
            debug!("Granting admin access: {new_admins}");

            // Really we want to set users.is_admin to true if their joined oauth_github.account_id
            // is in new_admin_ids, but diesel doesn't support inner_joins with updates yet:
            // <https://github.com/diesel-rs/diesel/issues/1478>
            // So instead, do 2 queries to first get the user ids, then do the update.
            let new_admin_user_account_ids = oauth_github::table
                .select((oauth_github::user_id, oauth_github::account_id))
                .filter(oauth_github::account_id.eq_any(&new_admin_ids))
                .get_results::<(i32, i64)>(&mut conn)
                .await?;
            let new_admin_user_ids: Vec<_> = new_admin_user_account_ids
                .iter()
                .map(|(user_id, _)| user_id)
                .collect();
            let updated_user_ids = diesel::update(users::table)
                .filter(users::id.eq_any(&new_admin_user_ids))
                .set(users::is_admin.eq(true))
                .returning(users::id)
                .get_results::<i32>(&mut conn)
                .await?;
            new_admin_user_account_ids
                .into_iter()
                .filter_map(|(user_id, account_id)| {
                    updated_user_ids.contains(&user_id).then_some(account_id)
                })
                .collect()
        };

        // New admins from the team repo that have been granted admin
        // access now.

        let added_admin_ids = HashSet::from_iter(added_admin_ids);
        if !added_admin_ids.is_empty() {
            let added_admins = format_repo_admins(&added_admin_ids).join(", ");
            info!("Granted admin access: {added_admins}");
        }

        // New admins from the team repo that don't have a crates.io
        // account yet.

        let skipped_new_admin_ids = new_admin_ids
            .difference(&added_admin_ids)
            .copied()
            .collect::<HashSet<_>>();

        if !skipped_new_admin_ids.is_empty() {
            let skipped_new_admins = format_repo_admins(&skipped_new_admin_ids).join(", ");
            info!("Skipped missing admins: {skipped_new_admins}");
        }

        // Existing admins from the database that are no longer in the
        // team repo.

        let obsolete_admin_ids = database_admin_ids
            .difference(&repo_admin_ids)
            .copied()
            .collect::<HashSet<_>>();

        let removed_admin_ids = if obsolete_admin_ids.is_empty() {
            Vec::new()
        } else {
            let obsolete_admins = format_database_admins(&obsolete_admin_ids).join(", ");
            debug!("Revoking admin access: {obsolete_admins}");

            // Similarly with the new admins, we have to do 2 queries here
            let removed_admin_user_account_ids = oauth_github::table
                .select((oauth_github::user_id, oauth_github::account_id))
                .filter(oauth_github::account_id.eq_any(&obsolete_admin_ids))
                .get_results::<(i32, i64)>(&mut conn)
                .await?;
            let removed_admin_user_ids: Vec<_> = removed_admin_user_account_ids
                .iter()
                .map(|(user_id, _)| user_id)
                .collect();
            let updated_user_ids = diesel::update(users::table)
                .filter(users::id.eq_any(&removed_admin_user_ids))
                .set(users::is_admin.eq(false))
                .returning(users::id)
                .get_results::<i32>(&mut conn)
                .await?;
            removed_admin_user_account_ids
                .into_iter()
                .filter_map(|(user_id, account_id)| {
                    updated_user_ids.contains(&user_id).then_some(account_id)
                })
                .collect()
        };

        let removed_admin_ids = HashSet::from_iter(removed_admin_ids);
        if !removed_admin_ids.is_empty() {
            let removed_admins = format_database_admins(&removed_admin_ids).join(", ");
            info!("Revoked admin access: {removed_admins}");
        }

        if added_admin_ids.is_empty() && removed_admin_ids.is_empty() {
            return Ok(());
        }

        let added_admins = format_repo_admins(&added_admin_ids);
        let removed_admins = format_database_admins(&removed_admin_ids);
        let context = context! { added_admins, removed_admins };

        for database_admin in &database_admins {
            let (github_id, login, email_address) = database_admin;
            if let Some(email_address) = email_address {
                if let Err(error) = send_email(&ctx, email_address, &context).await {
                    warn!(
                        "Failed to send email to admin {login} ({email_address}, github_id: {github_id}): {error:?}",
                    );
                }
            } else {
                warn!("No email address found for admin {login} (github_id: {github_id})",);
            }
        }

        Ok(())
    }
}

async fn send_email(
    ctx: &Environment,
    address: &str,
    context: &minijinja::Value,
) -> anyhow::Result<()> {
    let email = EmailMessage::from_template("admin_account", context);
    let email = email.context("Failed to render email template")?;
    let result = ctx.emails.send(address, email).await;
    result.context("Failed to send email")
}
