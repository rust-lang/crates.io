use crate::email::Email;
use crate::schema::{emails, users};
use crate::worker::Environment;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

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

        let format_repo_admins = |github_ids: &HashSet<i32>| {
            repo_admins
                .iter()
                .filter(|m| github_ids.contains(&m.github_id))
                .map(|m| format!("{} (github_id: {})", m.github, m.github_id))
                .collect::<Vec<_>>()
        };

        // Existing admins from the database.

        let database_admins = users::table
            .left_join(emails::table)
            .select((users::gh_id, users::gh_login, emails::email.nullable()))
            .filter(users::is_admin.eq(true))
            .get_results::<(i32, String, Option<String>)>(&mut conn)
            .await?;

        let database_admin_ids = database_admins
            .iter()
            .map(|(gh_id, _, _)| *gh_id)
            .collect::<HashSet<_>>();

        let format_database_admins = |github_ids: &HashSet<i32>| {
            database_admins
                .iter()
                .filter(|(gh_id, _, _)| github_ids.contains(gh_id))
                .map(|(gh_id, login, _)| format!("{} (github_id: {})", login, gh_id))
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

            diesel::update(users::table)
                .filter(users::gh_id.eq_any(&new_admin_ids))
                .set(users::is_admin.eq(true))
                .returning(users::gh_id)
                .get_results::<i32>(&mut conn)
                .await?
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

            diesel::update(users::table)
                .filter(users::gh_id.eq_any(&obsolete_admin_ids))
                .set(users::is_admin.eq(false))
                .returning(users::gh_id)
                .get_results::<i32>(&mut conn)
                .await?
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

        let email = AdminAccountEmail::new(added_admins, removed_admins);

        for database_admin in &database_admins {
            let (_, _, email_address) = database_admin;
            if let Some(email_address) = email_address {
                if let Err(error) = ctx.emails.send(email_address, email.clone()).await {
                    warn!(
                        "Failed to send email to admin {} ({}, github_id: {}): {}",
                        database_admin.1, email_address, database_admin.0, error
                    );
                }
            } else {
                warn!(
                    "No email address found for admin {} (github_id: {})",
                    database_admin.1, database_admin.0
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct AdminAccountEmail {
    added_admins: Vec<String>,
    removed_admins: Vec<String>,
}

impl AdminAccountEmail {
    fn new(added_admins: Vec<String>, removed_admins: Vec<String>) -> Self {
        Self {
            added_admins,
            removed_admins,
        }
    }
}

impl Email for AdminAccountEmail {
    fn subject(&self) -> String {
        "crates.io: Admin account changes".into()
    }

    fn body(&self) -> String {
        self.to_string()
    }
}

impl Display for AdminAccountEmail {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if !self.added_admins.is_empty() {
            writeln!(f, "Granted admin access:\n")?;
            for new_admin in &self.added_admins {
                writeln!(f, "- {}", new_admin)?;
            }
            writeln!(f)?;
        }

        if !self.removed_admins.is_empty() {
            writeln!(f, "Revoked admin access:")?;
            for obsolete_admin in &self.removed_admins {
                writeln!(f, "- {}", obsolete_admin)?;
            }
        }

        Ok(())
    }
}
