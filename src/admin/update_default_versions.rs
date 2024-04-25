use crate::models::update_default_version;
use crate::{db, schema::crates};
use anyhow::Context;
use diesel::prelude::*;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};

#[derive(clap::Parser, Debug)]
#[clap(
    name = "update-default-versions",
    about = "Iterates over every crate ever uploaded and updates the `default_versions` table."
)]
pub struct Opts;

pub fn run(_opts: Opts) -> anyhow::Result<()> {
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
        if let Err(error) = update_default_version(crate_id, &mut conn) {
            pb.suspend(|| warn!(%crate_id, %error, "Failed to update the default version"));
        }
    }

    Ok(())
}
