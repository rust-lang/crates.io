pub mod downloads;
pub mod metadata;
pub mod yank;

use axum::extract::{FromRequestParts, Path};
use diesel_async::AsyncPgConnection;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use utoipa::IntoParams;

use crate::models::{Crate, Version};
use crate::util::errors::{crate_not_found, AppResult};

#[derive(Deserialize, FromRequestParts, IntoParams)]
#[into_params(parameter_in = Path)]
#[from_request(via(Path))]
pub struct CrateVersionPath {
    /// Name of the crate
    pub name: String,
    /// Version number
    #[param(example = "1.0.0")]
    #[serde(deserialize_with = "deserialize_version")]
    pub version: String,
}

impl CrateVersionPath {
    pub async fn load_version(&self, conn: &mut AsyncPgConnection) -> AppResult<Version> {
        Ok(self.load_version_and_crate(conn).await?.0)
    }

    pub async fn load_version_and_crate(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> AppResult<(Version, Crate)> {
        version_and_crate(conn, &self.name, &self.version).await
    }
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

fn deserialize_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let s = String::deserialize(deserializer)?;
    let _ = semver::Version::parse(&s).map_err(Error::custom)?;
    Ok(s)
}
