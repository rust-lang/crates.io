pub mod deprecated;
pub mod downloads;
pub mod metadata;
pub mod yank;

use super::prelude::*;

use crate::models::{Crate, Version};

fn version_and_crate(
    conn: &PgConnection,
    crate_name: &str,
    semver: &str,
) -> AppResult<(Version, Crate)> {
    let krate: Crate = Crate::by_name(crate_name).first(conn)?;
    let version = krate.find_version(conn, semver)?;

    Ok((version, krate))
}
