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
        let repo_admin_github_ids = repo_admins
            .iter()
            .map(|m| m.github_id)
            .collect::<HashSet<i64>>();

        let mut conn = ctx.deadpool.get().await?;

        let format_repo_admins = |github_ids: &HashSet<i64>| {
            repo_admins
                .iter()
                .filter(|m| github_ids.contains(&m.github_id))
                .map(|m| format!("{} (github_id: {})", m.github, m.github_id))
                .collect::<Vec<_>>()
        };

        #[derive(Debug, HasQuery)]
        #[diesel(base_query = users::table.left_join(oauth_github::table).left_join(emails::table))]
        struct UserData {
            #[diesel(select_expression = users::id)]
            id: i32,
            #[diesel(select_expression = users::gh_login)]
            gh_login: String,
            #[diesel(select_expression = users::is_admin)]
            is_admin: bool,
            #[diesel(select_expression = oauth_github::account_id.nullable())]
            account_id: Option<i64>,
            #[diesel(select_expression = emails::email.nullable())]
            email: Option<String>,
        }

        // Fetch all database info for all accounts that are either currently marked as admins
        // or are admins according to the team repo.
        let database_user_data = UserData::query()
            .filter(
                users::is_admin
                    .eq(true)
                    .or(oauth_github::account_id.eq_any(&repo_admin_github_ids)),
            )
            .get_results(&mut conn)
            .await?;

        // All the relevant GitHub IDs we have in the database
        let database_user_github_ids = database_user_data
            .iter()
            .flat_map(|u| u.account_id)
            .collect::<HashSet<i64>>();

        let format_database_users = |user_ids: &HashSet<i32>| {
            database_user_data
                .iter()
                .filter(|u| user_ids.contains(&u.id))
                .map(|u| {
                    format!(
                        "{} (github_id: {})",
                        u.gh_login,
                        u.account_id
                            .map(|id| id.to_string())
                            .unwrap_or("None".into())
                    )
                })
                .collect::<Vec<_>>()
        };

        // New admins from the team repo that don't have admin access yet.
        let new_admin_user_ids = database_user_data
            .iter()
            .filter_map(|u| {
                (u.account_id
                    .is_some_and(|account_id| repo_admin_github_ids.contains(&account_id))
                    && !u.is_admin)
                    .then_some(u.id)
            })
            .collect::<HashSet<i32>>();

        let added_admin_user_ids = if new_admin_user_ids.is_empty() {
            Vec::new()
        } else {
            let new_admins = format_database_users(&new_admin_user_ids).join(", ");
            debug!("Granting admin access: {new_admins}");

            diesel::update(users::table)
                .filter(users::id.eq_any(&new_admin_user_ids))
                .set(users::is_admin.eq(true))
                .returning(users::id)
                .get_results::<i32>(&mut conn)
                .await?
        };

        // New admins from the team repo that have been granted admin
        // access now.
        let added_admin_user_ids = HashSet::from_iter(added_admin_user_ids);
        if !added_admin_user_ids.is_empty() {
            let added_admins = format_database_users(&added_admin_user_ids).join(", ");
            info!("Granted admin access: {added_admins}");
        }

        // New admins from the team repo that don't have a crates.io
        // account yet, so we can't find their GitHub ID in the database.
        let skipped_new_admin_github_ids = repo_admin_github_ids
            .difference(&database_user_github_ids)
            .copied()
            .collect::<HashSet<_>>();

        if !skipped_new_admin_github_ids.is_empty() {
            let skipped_new_admins = format_repo_admins(&skipped_new_admin_github_ids).join(", ");
            info!("Skipped missing admins: {skipped_new_admins}");
        }

        // Existing admins from the database that are no longer in the
        // team repo.
        let obsolete_admin_user_ids = database_user_data
            .iter()
            .filter_map(|u| {
                (u.is_admin
                    && u.account_id
                        .is_none_or(|account_id| !repo_admin_github_ids.contains(&account_id)))
                .then_some(u.id)
            })
            .collect::<HashSet<i32>>();

        let removed_admin_user_ids = if obsolete_admin_user_ids.is_empty() {
            Vec::new()
        } else {
            let obsolete_admins = format_database_users(&obsolete_admin_user_ids).join(", ");
            debug!("Revoking admin access: {obsolete_admins}");

            diesel::update(users::table)
                .filter(users::id.eq_any(&obsolete_admin_user_ids))
                .set(users::is_admin.eq(false))
                .returning(users::id)
                .get_results::<i32>(&mut conn)
                .await?
        };

        let removed_admin_user_ids = HashSet::from_iter(removed_admin_user_ids);
        if !removed_admin_user_ids.is_empty() {
            let removed_admins = format_database_users(&removed_admin_user_ids).join(", ");
            info!("Revoked admin access: {removed_admins}");
        }

        if added_admin_user_ids.is_empty() && removed_admin_user_ids.is_empty() {
            return Ok(());
        }

        let added_admins = format_database_users(&added_admin_user_ids);
        let removed_admins = format_database_users(&removed_admin_user_ids);
        let context = context! { added_admins, removed_admins };

        // Attempt to notify admins that were in the database previously of any changes via email.
        for database_admin in database_user_data.iter().filter(|u| u.is_admin) {
            let UserData {
                account_id,
                gh_login,
                email,
                ..
            } = database_admin;
            if let Some(email_address) = email {
                if let Err(error) = send_email(&ctx, email_address, &context).await {
                    warn!(
                        "Failed to send email to admin {gh_login} \
                        ({email_address}, github_id: {account_id:?}): {error:?}"
                    );
                }
            } else {
                warn!("No email address found for admin {gh_login} (github_id: {account_id:?})",);
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
