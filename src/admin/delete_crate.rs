use crate::background_jobs::Job;
use crate::{admin::dialoguer, config, db, models::Crate, schema::crates};

use diesel::prelude::*;
use reqwest::blocking::Client;

#[derive(clap::Parser, Debug)]
#[command(
    name = "delete-crate",
    about = "Purge all references to a crate from the database.",
    after_help = "Please be super sure you want to do this before running this!"
)]
pub struct Opts {
    /// Name of the crate
    crate_name: String,

    /// Don't ask for confirmation: yes, we are sure. Best for scripting.
    #[arg(short, long)]
    yes: bool,
}

pub fn run(opts: Opts) {
    let conn = &mut db::oneoff_connection().unwrap();
    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        delete(opts, conn);
        Ok(())
    })
    .unwrap()
}

fn delete(opts: Opts, conn: &mut PgConnection) {
    let krate: Crate = Crate::by_name(&opts.crate_name).first(conn).unwrap();

    let config = config::Base::from_environment();
    let uploader = config.uploader();
    let client = Client::new();

    if !opts.yes {
        let prompt = format!(
            "Are you sure you want to delete {} ({})?",
            opts.crate_name, krate.id
        );
        if !dialoguer::confirm(&prompt) {
            return;
        }
    }

    println!("deleting the crate");
    let n = diesel::delete(crates::table.find(krate.id))
        .execute(conn)
        .unwrap();
    println!("  {n} deleted");

    if !opts.yes && !dialoguer::confirm("commit?") {
        panic!("aborting transaction");
    }

    if dotenv::var("FEATURE_INDEX_SYNC").is_ok() {
        Job::enqueue_sync_to_index(&krate.name, conn).unwrap();
    } else {
        uploader.delete_index(&client, &krate.name).unwrap();
    }
}
