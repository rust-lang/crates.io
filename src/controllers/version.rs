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

fn extract_crate_name_and_semver(req: &dyn RequestExt) -> AppResult<(&str, &str)> {
    let name = extract_crate_name(req);
    let version = extract_semver(req)?;
    Ok((name, version))
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
