pub mod authors;
pub mod dependencies;
pub mod docs;
pub mod downloads;
pub mod metadata;
pub mod readme;
pub mod update;
pub mod yank;

use axum::extract::{FromRequestParts, Path};
use crates_io_database::canon_crate_name;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use utoipa::IntoParams;

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
        let row = Self::base_query(&self.name, &self.version)
            .select((crates::id, Option::<Version>::as_select()))
            .first::<(i32, _)>(&mut conn)
            .await
            .optional()?;

        self.gather(row).map(|r| r.0)
    }

    pub async fn load_version_and_crate(
        &self,
        mut conn: &AsyncPgConnection,
    ) -> AppResult<(Version, Crate)> {
        let row = Self::base_query(&self.name, &self.version)
            .select(<(Crate, Option<Version>)>::as_select())
            .first(&mut conn)
            .await
            .optional()?;

        self.gather(row)
    }

    #[diesel::dsl::auto_type(no_type_alias)]
    fn base_query<'a>(crate_name: &'a str, semver: &'a str) -> _ {
        crates::table
            .left_join(
                versions::table.on(crates::id
                    .eq(versions::crate_id)
                    .and(versions::num.eq(semver))),
            )
            .filter(canon_crate_name(crates::name).eq(canon_crate_name(crate_name)))
    }

    fn gather<C, V>(&self, row: Option<(C, Option<V>)>) -> AppResult<(V, C)> {
        let (krate_or_id, version) = row.ok_or_else(|| crate_not_found(&self.name))?;
        let version = version.ok_or_else(|| version_not_found(&self.name, &self.version))?;
        Ok((version, krate_or_id))
    }
}

fn deserialize_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let s = String::deserialize(deserializer)?;
    let _ = semver::Version::parse(&s).map_err(Error::custom)?;
    Ok(s)
}
