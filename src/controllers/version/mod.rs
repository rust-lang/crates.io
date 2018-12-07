pub mod deprecated;
pub mod downloads;
pub mod metadata;
pub mod yank;

use super::prelude::*;

use crate::models::{Crate, CrateVersions, Version};
use crate::schema::versions;

fn version_and_crate(req: &mut dyn Request) -> CargoResult<(Version, Crate)> {
    let crate_name = &req.params()["crate_id"];
    let semver = &req.params()["version"];
    if semver::Version::parse(semver).is_err() {
        return Err(human(&format_args!("invalid semver: {}", semver)));
    };
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;
    let version = krate
        .all_versions()
        .filter(versions::num.eq(semver))
        .first(&*conn)
        .map_err(|_| {
            human(&format_args!(
                "crate `{}` does not have a version `{}`",
                crate_name, semver
            ))
        })?;
    Ok((version, krate))
}
