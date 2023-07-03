use crate::background_jobs::Job;
use crate::schema::crates;
use crate::{admin::dialoguer, db, schema::versions};
use anyhow::Context;
use diesel::prelude::*;

#[derive(clap::Parser, Debug)]
#[command(
    name = "delete-version",
    about = "Purge all references to a crate's version from the database.",
    after_help = "Please be super sure you want to do this before running this!"
)]
pub struct Opts {
    /// Name of the crate
    crate_name: String,

    /// Version numbers that should be deleted
    #[arg(value_name = "VERSION", required = true)]
    versions: Vec<String>,

    /// Don't ask for confirmation: yes, we are sure. Best for scripting.
    #[arg(short, long)]
    yes: bool,
}

pub fn run(opts: Opts) {
    let crate_name = &opts.crate_name;

    let conn = &mut db::oneoff_connection()
        .context("Failed to establish database connection")
        .unwrap();

    let crate_id: i32 = crates::table
        .select(crates::id)
        .filter(crates::name.eq(crate_name))
        .first(conn)
        .context("Failed to look up crate id from the database")
        .unwrap();

    println!("Deleting the following versions of the `{crate_name}` crate:");
    println!();
    for version in &opts.versions {
        println!(" - {version}");
    }
    println!();

    if !opts.yes && !dialoguer::confirm("Do you want to permanently delete these versions?") {
        return;
    }

    info!(%crate_name, %crate_id, versions = ?opts.versions, "Deleting versions from the database");
    let result = diesel::delete(
        versions::table
            .filter(versions::crate_id.eq(crate_id))
            .filter(versions::num.eq_any(&opts.versions)),
    )
    .execute(conn);

    match result {
        Ok(num_deleted) if num_deleted == opts.versions.len() => {}
        Ok(num_deleted) => {
            warn!(
                %crate_name,
                "Deleted only {num_deleted} of {num_expected} versions from the database",
                num_expected = opts.versions.len()
            );
        }
        Err(error) => {
            warn!(%crate_name, ?error, "Failed to delete versions from the database")
        }
    }

    info!(%crate_name, "Enqueuing index sync jobs");
    if let Err(error) = Job::enqueue_sync_to_index(crate_name, conn) {
        warn!(%crate_name, ?error, "Failed to enqueue index sync jobs");
    }
}
