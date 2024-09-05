use crate::schema::{crate_owners, teams, users};
use crate::storage::{FeedId, Storage};
use crate::worker::jobs;
use crate::{admin::dialoguer, db, schema::crates};
use anyhow::Context;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::Text;
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

pub fn run(opts: Opts) -> anyhow::Result<()> {
    let conn = &mut db::oneoff_connection().context("Failed to establish database connection")?;

    let store = Storage::from_environment();

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
        .load::<(String, i32, String)>(conn)
        .context("Failed to look up crate name from the database")?;

    let mut existing_crates: HashMap<String, (i32, Vec<String>)> = HashMap::new();
    for (name, id, login) in query_result {
        let entry = existing_crates
            .entry(name)
            .or_insert_with(|| (id, Vec::new()));

        entry.1.push(login);
    }

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

    if !opts.yes && !dialoguer::confirm("Do you want to permanently delete these crates?") {
        return Ok(());
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to initialize tokio runtime")?;

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

        info!("{name}: Deleting crate files from S3…");
        if let Err(error) = rt.block_on(store.delete_all_crate_files(name)) {
            warn!("{name}: Failed to delete crate files from S3: {error}");
        }

        info!("{name}: Deleting readme files from S3…");
        if let Err(error) = rt.block_on(store.delete_all_readmes(name)) {
            warn!("{name}: Failed to delete readme files from S3: {error}");
        }

        info!("{name}: Deleting RSS feed from S3…");
        let feed_id = FeedId::Crate { name };
        if let Err(error) = rt.block_on(store.delete_feed(&feed_id)) {
            warn!("{name}: Failed to delete RSS feed from S3: {error}");
        }
    }

    Ok(())
}
