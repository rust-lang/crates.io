pub mod authors;
pub mod dependencies;
pub mod docs;
pub mod downloads;
pub mod metadata;
pub mod readme;
pub mod update;
pub mod yank;

use axum::extract::{FromRequestParts, Path};
use crates_io_diesel_helpers::canon_crate_name;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use utoipa::IntoParams;

use crate::controllers::krate::load_crate;
use crate::models::{Crate, Version};
use crate::schema::{crates, versions};
use crate::util::errors::{AppResult, crate_not_found, version_not_found};

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
    pub async fn load_version(&self, mut conn: &AsyncPgConnection) -> AppResult<Version> {
        let (_, version) = crates::table
            .left_join(
                versions::table.on(crates::id
                    .eq(versions::crate_id)
                    .and(versions::num.eq(&self.version))),
            )
            .filter(canon_crate_name(crates::name).eq(&self.name))
            .select((crates::id, Option::<Version>::as_select()))
            .first::<(i32, _)>(&mut conn)
            .await
            .optional()?
            .ok_or_else(|| crate_not_found(&self.name))?;
        let version = version.ok_or_else(|| version_not_found(&self.name, &self.version))?;
        Ok(version)
    }

    pub async fn load_version_and_crate(
        &self,
        conn: &AsyncPgConnection,
    ) -> AppResult<(Version, Crate)> {
        version_and_crate(conn, &self.name, &self.version).await
    }
}

async fn version_and_crate(
    conn: &AsyncPgConnection,
    crate_name: &str,
    semver: &str,
) -> AppResult<(Version, Crate)> {
    let krate = load_crate(conn, crate_name).await?;
    let version = krate
        .find_version(conn, semver)
        .await?
        .ok_or_else(|| version_not_found(crate_name, semver))?;

    Ok((version, krate))
}

fn deserialize_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let s = String::deserialize(deserializer)?;
    let _ = semver::Version::parse(&s).map_err(Error::custom)?;
    Ok(s)
}
