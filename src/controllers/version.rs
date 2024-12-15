pub mod downloads;
pub mod metadata;
pub mod yank;

use axum::extract::{FromRequestParts, Path};
use diesel_async::AsyncPgConnection;

use crate::models::{Crate, Version};
use crate::util::errors::{crate_not_found, AppResult};

#[derive(Deserialize, FromRequestParts)]
#[from_request(via(Path))]
pub struct CrateVersionPath {
    /// Name of the crate
    pub name: String,
    /// Version number
    pub version: String,
}

async fn version_and_crate(
    conn: &mut AsyncPgConnection,
    crate_name: &str,
    semver: &str,
) -> AppResult<(Version, Crate)> {
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    let krate: Crate = Crate::by_name(crate_name)
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(crate_name))?;

    let version = krate.find_version(conn, semver).await?;

    Ok((version, krate))
}
