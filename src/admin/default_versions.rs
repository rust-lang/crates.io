use crate::models::{update_default_version, verify_default_version};
use crate::{db, schema::crates};
use anyhow::Context;
use diesel::prelude::*;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};

#[derive(clap::Parser, Debug, Eq, PartialEq)]
#[clap(
    name = "default-versions",
    about = "Iterates over every crate ever uploaded and updates or verifies the contents of the `default_versions` table."
)]
pub enum Command {
    Update,
    Verify,
}

pub fn run(command: Command) -> anyhow::Result<()> {
    let mut conn = db::oneoff_connection().context("Failed to connect to the database")?;

    let crate_ids: Vec<i32> = crates::table
        .select(crates::id)
        .load(&mut conn)
        .context("Failed to load crates")?;

    let pb = ProgressBar::new(crate_ids.len() as u64);
    pb.set_style(ProgressStyle::with_template(
        "{bar:60} ({pos}/{len}, ETA {eta})",
    )?);

    for crate_id in crate_ids.into_iter().progress_with(pb.clone()) {
        let func = match command {
            Command::Update => update_default_version,
            Command::Verify => verify_default_version,
        };

        if let Err(error) = func(crate_id, &mut conn) {
            pb.suspend(|| warn!(%crate_id, %error, "Failed to update the default version"));
        }
    }

    Ok(())
}
