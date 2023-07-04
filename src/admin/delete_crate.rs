use crate::background_jobs::Job;
use crate::{admin::dialoguer, db, schema::crates};
use anyhow::Context;
use std::collections::HashMap;

use diesel::prelude::*;

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

pub fn run(opts: Opts) {
    let conn = &mut db::oneoff_connection()
        .context("Failed to establish database connection")
        .unwrap();

    let mut crate_names = opts.crate_names;
    crate_names.sort();

    let existing_crates = crates::table
        .select((crates::name, crates::id))
        .filter(crates::name.eq_any(&crate_names))
        .load(conn)
        .context("Failed to look up crate name from the database")
        .unwrap();

    let existing_crates: HashMap<String, i32> = existing_crates.into_iter().collect();

    println!("Deleting the following crates:");
    println!();
    for name in &crate_names {
        match existing_crates.get(name) {
            Some(id) => println!(" - {name} (id={id})"),
            None => println!(" - {name} (⚠️ crate not found)"),
        }
    }
    println!();

    if !opts.yes && !dialoguer::confirm("Do you want to permanently delete these crates?") {
        return;
    }

    for name in &crate_names {
        if let Some(id) = existing_crates.get(name) {
            info!(%name, "Deleting crate from the database");
            if let Err(error) = diesel::delete(crates::table.find(id)).execute(conn) {
                warn!(%name, %id, ?error, "Failed to delete crate from the database");
            }
        } else {
            info!(%name, "Skipping missing crate");
        };

        info!(%name, "Enqueuing index sync jobs");
        if let Err(error) = Job::enqueue_sync_to_index(name, conn) {
            warn!(%name, ?error, "Failed to enqueue index sync jobs");
        }
    }
}
