use crate::models;
use crate::util::diesel::Conn;
use anyhow::Context;
use diesel::prelude::*;
use sentry::Level;

#[instrument(skip_all, fields(krate.name = ?name))]
pub fn get_index_data(name: &str, conn: &mut impl Conn) -> anyhow::Result<Option<String>> {
    debug!("Looking up crate by name");
    let Some(krate): Option<models::Crate> =
        models::Crate::by_exact_name(name).first(conn).optional()?
    else {
        return Ok(None);
    };

    debug!("Gathering remaining index data");
    let crates = krate
        .index_metadata(conn)
        .context("Failed to gather index metadata")?;

    // This can sometimes happen when we delete versions upon owner request
    // but don't realize that the crate is now left with no versions at all.
    //
    // In this case we will delete the crate from the index and log a warning to
    // Sentry to clean this up in the database.
    if crates.is_empty() {
        let message = format!("Crate `{name}` has no versions left");
        sentry::capture_message(&message, Level::Warning);

        return Ok(None);
    }

    debug!("Serializing index data");
    let mut bytes = Vec::new();
    crates_io_index::write_crates(&crates, &mut bytes)
        .context("Failed to serialize index metadata")?;

    let str = String::from_utf8(bytes).context("Failed to decode index metadata as utf8")?;

    Ok(Some(str))
}
