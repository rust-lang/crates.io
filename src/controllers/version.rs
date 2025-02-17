pub mod authors;
pub mod dependencies;
pub mod downloads;
pub mod metadata;
pub mod readme;
pub mod update;
pub mod yank;

use axum::extract::{FromRequestParts, Path};
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
    use ext::*;

    let (krate, version) = crate_and_version_query(crate_name, semver)
        .select(<(Crate, Option<Version>)>::as_select())
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(crate_name))?;

    let version = version.ok_or_else(|| version_not_found(crate_name, semver))?;
    Ok((version, krate))
}

fn deserialize_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let s = String::deserialize(deserializer)?;
    let _ = semver::Version::parse(&s).map_err(Error::custom)?;
    Ok(s)
}

mod ext {
    use super::*;
    use crates_io_diesel_helpers::canon_crate_name;

    #[diesel::dsl::auto_type()]
    pub fn crate_and_version_query<'a>(crate_name: &'a str, semver: &'a str) -> _ {
        crates::table
            .left_join(
                versions::table.on(crates::id
                    .eq(versions::crate_id)
                    .and(versions::num.eq(semver))),
            )
            .filter(canon_crate_name(crates::name).eq(canon_crate_name(crate_name)))
    }

    pub trait CrateVersionPathExt {
        fn crate_and_version(&self) -> crate_and_version_query<'_>;
    }

    impl CrateVersionPathExt for CrateVersionPath {
        fn crate_and_version(&self) -> crate_and_version_query<'_> {
            crate_and_version_query(&self.name, &self.version)
        }
    }
}
