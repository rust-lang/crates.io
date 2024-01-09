use crate::email::Email;
use crate::schema::{emails, users};
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel::RunQueryDsl;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

/// See <https://github.com/rust-lang/team/blob/master/teams/crates-io-admins.toml>.
const TEAM_NAME: &str = "crates-io-admins";

#[derive(Serialize, Deserialize)]
pub struct SyncAdmins;

impl BackgroundJob for SyncAdmins {
    const JOB_NAME: &'static str = "sync_admins";

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        info!("Syncing admins from rust-lang/team repo…");

        let repo_admins = ctx.team_repo.get_team(TEAM_NAME).await?.members;
        let repo_admin_ids = repo_admins
            .iter()
            .map(|m| m.github_id)
            .collect::<HashSet<_>>();

        spawn_blocking::<_, _, anyhow::Error>(move || {
            let mut conn = ctx.connection_pool.get()?;

            let database_admins = users::table
                .left_join(emails::table)
                .select((users::gh_id, users::gh_login, emails::email.nullable()))
                .filter(users::is_admin.eq(true))
                .get_results::<(i32, String, Option<String>)>(&mut conn)?;

            let database_admin_ids = database_admins
                .iter()
                .map(|(gh_id, _, _)| *gh_id)
                .collect::<HashSet<_>>();

            let new_admin_ids = repo_admin_ids
                .difference(&database_admin_ids)
                .collect::<HashSet<_>>();

            let new_admins = if new_admin_ids.is_empty() {
                debug!("No new admins to add");
                vec![]
            } else {
                let new_admins = repo_admins
                    .iter()
                    .filter(|m| new_admin_ids.contains(&&m.github_id))
                    .map(|m| format!("{} (github_id: {})", m.github, m.github_id))
                    .collect::<Vec<_>>();

                info!("Adding new admins: {}", new_admins.join(", "));

                diesel::update(users::table)
                    .filter(users::gh_id.eq_any(new_admin_ids))
                    .set(users::is_admin.eq(true))
                    .execute(&mut conn)?;

                new_admins
            };

            let obsolete_admin_ids = database_admin_ids
                .difference(&repo_admin_ids)
                .collect::<HashSet<_>>();

            let obsolete_admins = if obsolete_admin_ids.is_empty() {
                debug!("No obsolete admins to remove");
                vec![]
            } else {
                let obsolete_admins = database_admins
                    .iter()
                    .filter(|(gh_id, _, _)| obsolete_admin_ids.contains(&gh_id))
                    .map(|(gh_id, login, _)| format!("{} (github_id: {})", login, gh_id))
                    .collect::<Vec<_>>();

                info!("Removing obsolete admins: {}", obsolete_admins.join(", "));

                diesel::update(users::table)
                    .filter(users::gh_id.eq_any(obsolete_admin_ids))
                    .set(users::is_admin.eq(false))
                    .execute(&mut conn)?;

                obsolete_admins
            };

            if !new_admins.is_empty() || !obsolete_admins.is_empty() {
                let email = AdminAccountEmail::new(new_admins, obsolete_admins);

                for database_admin in &database_admins {
                    let (_, _, email_address) = database_admin;
                    if let Some(email_address) = email_address {
                        if let Err(error) = ctx.emails.send(email_address, email.clone()) {
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
            }

            Ok(())
        })
        .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct AdminAccountEmail {
    new_admins: Vec<String>,
    obsolete_admins: Vec<String>,
}

impl AdminAccountEmail {
    fn new(new_admins: Vec<String>, obsolete_admins: Vec<String>) -> Self {
        Self {
            new_admins,
            obsolete_admins,
        }
    }
}

impl Email for AdminAccountEmail {
    const SUBJECT: &'static str = "crates.io: Admin account changes";

    fn body(&self) -> String {
        self.to_string()
    }
}

impl Display for AdminAccountEmail {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if !self.new_admins.is_empty() {
            writeln!(f, "New admins have been added:\n")?;
            for new_admin in &self.new_admins {
                writeln!(f, "- {}", new_admin)?;
            }
            writeln!(f)?;
        }

        if !self.obsolete_admins.is_empty() {
            writeln!(f, "Admin access has been revoked for:")?;
            for obsolete_admin in &self.obsolete_admins {
                writeln!(f, "- {}", obsolete_admin)?;
            }
        }

        Ok(())
    }
}
