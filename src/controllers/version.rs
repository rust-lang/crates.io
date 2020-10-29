pub mod deprecated;
pub mod downloads;
pub mod metadata;
pub mod yank;

use super::prelude::*;

use crate::db::DieselPooledConn;
use crate::models::{Crate, Version};

fn version_and_crate(req: &dyn RequestExt) -> AppResult<(DieselPooledConn<'_>, Version, Crate)> {
    let crate_name = extract_crate_name(req);
    let semver = extract_semver(req)?;

    let conn = req.db_conn()?;
    let krate: Crate = Crate::by_name(crate_name).first(&*conn)?;
    let version = krate.find_version(&conn, semver)?;

    Ok((conn, version, krate))
}

fn extract_crate_name(req: &dyn RequestExt) -> &str {
    &req.params()["crate_id"]
}

fn extract_semver(req: &dyn RequestExt) -> AppResult<&str> {
    let semver = &req.params()["version"];
    if semver::Version::parse(semver).is_err() {
        return Err(cargo_err(&format_args!("invalid semver: {}", semver)));
    };
    Ok(semver)
}
