use crate::schema::users;
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel::RunQueryDsl;
use std::collections::HashSet;
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
                .select((users::gh_id, users::gh_login))
                .filter(users::is_admin.eq(true))
                .get_results::<(i32, String)>(&mut conn)?;

            let database_admin_ids = database_admins
                .iter()
                .map(|(gh_id, _)| *gh_id)
                .collect::<HashSet<_>>();

            let new_admin_ids = repo_admin_ids
                .difference(&database_admin_ids)
                .collect::<HashSet<_>>();

            if new_admin_ids.is_empty() {
                debug!("No new admins to add");
            } else {
                let new_admins = repo_admins
                    .iter()
                    .filter(|m| new_admin_ids.contains(&&m.github_id))
                    .map(|m| format!("{} (github_id: {})", m.github, m.github_id))
                    .collect::<Vec<_>>()
                    .join(", ");

                info!("Adding new admins: {}", new_admins);

                diesel::update(users::table)
                    .filter(users::gh_id.eq_any(new_admin_ids))
                    .set(users::is_admin.eq(true))
                    .execute(&mut conn)?;
            }

            let obsolete_admin_ids = database_admin_ids
                .difference(&repo_admin_ids)
                .collect::<HashSet<_>>();

            if obsolete_admin_ids.is_empty() {
                debug!("No obsolete admins to remove");
            } else {
                let obsolete_admins = database_admins
                    .iter()
                    .filter(|(gh_id, _)| obsolete_admin_ids.contains(&gh_id))
                    .map(|(gh_id, login)| format!("{} (github_id: {})", login, gh_id))
                    .collect::<Vec<_>>()
                    .join(", ");

                info!("Removing obsolete admins: {}", obsolete_admins);

                diesel::update(users::table)
                    .filter(users::gh_id.eq_any(obsolete_admin_ids))
                    .set(users::is_admin.eq(false))
                    .execute(&mut conn)?;
            }

            Ok(())
        })
        .await?;

        Ok(())
    }
}
