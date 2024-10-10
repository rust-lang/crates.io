use crate::schema::{crate_owners, teams, users};
use crate::tasks::spawn_blocking;
use crate::worker::jobs;
use crate::{admin::dialoguer, db, schema::crates};
use anyhow::Context;
use crates_io_worker::BackgroundJob;
use diesel::dsl::sql;
use diesel::sql_types::Text;
use diesel::{ExpressionMethods, JoinOnDsl, QueryDsl};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use std::collections::HashMap;

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

    let query_result = {
        use diesel_async::RunQueryDsl;

        crates::table
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
            .load::<(String, i32, String)>(&mut conn).await
            .context("Failed to look up crate name from the database")
    }?;

    let mut existing_crates: HashMap<String, (i32, Vec<String>)> = HashMap::new();
    for (name, id, login) in query_result {
        let entry = existing_crates
            .entry(name)
            .or_insert_with(|| (id, Vec::new()));

        entry.1.push(login);
    }

    spawn_blocking(move || {
        use diesel::RunQueryDsl;

        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        println!("Deleting the following crates:");
        println!();
        for name in &crate_names {
            match existing_crates.get(name) {
                Some((id, owners)) => {
                    let owners = owners.join(", ");
                    println!(" - {name} (id={id}, owners={owners})");
                }
                None => println!(" - {name} (⚠️ crate not found)"),
            }
        }
        println!();

        if !opts.yes && !dialoguer::confirm("Do you want to permanently delete these crates?")? {
            return Ok(());
        }

        for name in &crate_names {
            if let Some((id, _)) = existing_crates.get(name) {
                info!("{name}: Deleting crate from the database…");
                if let Err(error) = diesel::delete(crates::table.find(id)).execute(conn) {
                    warn!(%id, "{name}: Failed to delete crate from the database: {error}");
                }
            } else {
                info!("{name}: Skipped missing crate");
            };

            info!("{name}: Enqueuing index sync jobs…");
            if let Err(error) = jobs::enqueue_sync_to_index(name, conn) {
                warn!("{name}: Failed to enqueue index sync jobs: {error}");
            }

            info!("{name}: Enqueuing DeleteCrateFromStorage job…");
            let job = jobs::DeleteCrateFromStorage::new(name.into());
            if let Err(error) = job.enqueue(conn) {
                warn!("{name}: Failed to enqueue DeleteCrateFromStorage job: {error}");
            }
        }

        Ok(())
    })
    .await
}
