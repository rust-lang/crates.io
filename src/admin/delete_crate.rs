use crate::schema::{crate_owners, teams, users};
use crate::worker::jobs;
use crate::{admin::dialoguer, db, schema::crates};
use anyhow::Context;
use colored::Colorize;
use crates_io_worker::BackgroundJob;
use diesel::dsl::sql;
use diesel::sql_types::Text;
use diesel::{ExpressionMethods, JoinOnDsl, QueryDsl};
use diesel_async::RunQueryDsl;
use std::collections::HashMap;
use std::fmt::Display;

#[derive(clap::Parser, Debug)]
#[command(
    name = "delete-crate",
    about = "Purge all references to a crate from the database.",
    after_help = "Please be super sure you want to do this before running this!"
)]
pub struct Opts {
    /// Names of the crates
    #[arg(value_name = "NAME", required = true)]
    crate_names: Vec<String>,

    /// Don't ask for confirmation: yes, we are sure. Best for scripting.
    #[arg(short, long)]
    yes: bool,
}

pub async fn run(opts: Opts) -> anyhow::Result<()> {
    let mut conn = db::oneoff_async_connection()
        .await
        .context("Failed to establish database connection")?;

    let mut crate_names = opts.crate_names;
    crate_names.sort();

    let query_result = crates::table
        .select((
            crates::name,
            crates::id,
            sql::<Text>(
                "CASE WHEN crate_owners.owner_kind = 1 THEN teams.login ELSE users.gh_login END",
            ),
        ))
        .left_join(crate_owners::table.on(crate_owners::crate_id.eq(crates::id)))
        .left_join(teams::table.on(teams::id.eq(crate_owners::owner_id)))
        .left_join(users::table.on(users::id.eq(crate_owners::owner_id)))
        .filter(crates::name.eq_any(&crate_names))
        .load::<(String, i32, String)>(&mut conn)
        .await
        .context("Failed to look up crate name from the database")?;

    let mut existing_crates: HashMap<String, CrateInfo> = HashMap::new();
    for (name, id, login) in query_result {
        let entry = existing_crates
            .entry(name)
            .or_insert_with(|| CrateInfo::new(id));

        entry.owners.push(login);
    }

    println!("Deleting the following crates:");
    println!();
    for name in &crate_names {
        match existing_crates.get(name) {
            Some(info) => println!(" - {} ({info})", name.bold()),
            None => println!(" - {name} (⚠️ crate not found)"),
        }
    }
    println!();

    if !opts.yes
        && !dialoguer::async_confirm("Do you want to permanently delete these crates?").await?
    {
        return Ok(());
    }

    for name in &crate_names {
        if let Some(crate_info) = existing_crates.get(name) {
            let id = crate_info.id;

            info!("{name}: Deleting crate from the database…");
            if let Err(error) = diesel::delete(crates::table.find(id))
                .execute(&mut conn)
                .await
            {
                warn!(%id, "{name}: Failed to delete crate from the database: {error}");
            }
        } else {
            info!("{name}: Skipped missing crate");
        };

        info!("{name}: Enqueuing index sync jobs…");
        let job = jobs::SyncToGitIndex::new(name);
        if let Err(error) = job.async_enqueue(&mut conn).await {
            warn!("{name}: Failed to enqueue SyncToGitIndex job: {error}");
        }

        let job = jobs::SyncToSparseIndex::new(name);
        if let Err(error) = job.async_enqueue(&mut conn).await {
            warn!("{name}: Failed to enqueue SyncToSparseIndex job: {error}");
        }

        info!("{name}: Enqueuing DeleteCrateFromStorage job…");
        let job = jobs::DeleteCrateFromStorage::new(name.into());
        if let Err(error) = job.async_enqueue(&mut conn).await {
            warn!("{name}: Failed to enqueue DeleteCrateFromStorage job: {error}");
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct CrateInfo {
    id: i32,
    owners: Vec<String>,
}

impl CrateInfo {
    pub fn new(id: i32) -> Self {
        let owners = Vec::with_capacity(1);
        Self { id, owners }
    }
}

impl Display for CrateInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let id = self.id;
        let owners = self.owners.join(", ");
        write!(f, "id={id}, owners={owners}")
    }
}
