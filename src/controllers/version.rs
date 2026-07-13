pub mod authors;
pub mod dependencies;
pub mod docs;
pub mod downloads;
pub mod metadata;
pub mod readme;
pub mod update;
pub mod yank;

use axum::extract::{FromRequestParts, Path};
use crates_io_database::fns::canon_crate_name;
use crates_io_validation::validate_crate_name;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use http::request::Parts;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use utoipa::IntoParams;

use crate::models::{Crate, Version};
use crate::schema::{crates, versions};
use crate::util::errors::{AppResult, BoxedAppError, crate_not_found, custom, version_not_found};

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub struct CrateVersionPath {
    /// Name of the crate
    pub name: String,
    /// Version number
    #[param(example = "1.0.0")]
    #[serde(deserialize_with = "deserialize_version")]
    pub version: String,
}

impl<S: Send + Sync> FromRequestParts<S> for CrateVersionPath {
    type Rejection = BoxedAppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(path) = Path::<CrateVersionPath>::from_request_parts(parts, state)
            .await
            .map_err(|err| custom(err.status(), err.body_text()))?;

        // If the name is not a valid crate name it cannot exist in the
        // database, so we skip the lookup and return a regular "not found"
        // response. This also avoids passing invalid input (e.g. names
        // containing null bytes) to the database layer, where PostgreSQL would
        // reject the query with a confusing `invalid byte sequence for encoding
        // "UTF8": 0x00` error and cause a 500 response. (The version is already
        // validated as valid semver during deserialization.)
        if validate_crate_name("crate", &path.name).is_err() {
            return Err(crate_not_found(&path.name));
        }

        Ok(path)
    }
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
