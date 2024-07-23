pub mod downloads;
pub mod metadata;
pub mod yank;

use super::prelude::*;

use crate::models::{Crate, Version};
use crate::util::diesel::Conn;
use crate::util::errors::crate_not_found;

fn version_and_crate(
    conn: &mut impl Conn,
    crate_name: &str,
    semver: &str,
) -> AppResult<(Version, Crate)> {
    let krate: Crate = Crate::by_name(crate_name)
        .first(conn)
        .optional()?
        .ok_or_else(|| crate_not_found(crate_name))?;

    let version = krate.find_version(conn, semver)?;

    Ok((version, krate))
}
