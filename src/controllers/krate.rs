use crate::models::Crate;
use crate::util::errors::{AppResult, BoxedAppError, crate_not_found, custom};
use axum::extract::{FromRequestParts, Path};
use crates_io_database::schema::crates;
use crates_io_validation::validate_crate_name;
use diesel::{OptionalExtension, QueryDsl};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use http::request::Parts;
use serde::Deserialize;
use utoipa::IntoParams;

pub mod delete;
pub mod downloads;
pub mod follow;
pub mod metadata;
pub mod owners;
pub mod publish;
pub mod rev_deps;
pub mod search;
pub mod update;
pub mod versions;

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub struct CratePath {
    /// Name of the crate
    pub name: String,
}

impl<S: Send + Sync> FromRequestParts<S> for CratePath {
    type Rejection = BoxedAppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(path) = Path::<CratePath>::from_request_parts(parts, state)
            .await
            .map_err(|err| custom(err.status(), err.body_text()))?;

        // If the name is not a valid crate name it cannot exist in the
        // database, so we skip the lookup and return a regular "not found"
        // response. This also avoids passing invalid input (e.g. names
        // containing null bytes) to the database layer, where PostgreSQL would
        // reject the query with a confusing `invalid byte sequence for encoding
        // "UTF8": 0x00` error and cause a 500 response.
        if validate_crate_name("crate", &path.name).is_err() {
            return Err(crate_not_found(&path.name));
        }

        Ok(path)
    }
}

impl CratePath {
    pub async fn load_crate(&self, conn: &AsyncPgConnection) -> AppResult<Crate> {
        load_crate(conn, &self.name).await
    }

    pub async fn load_crate_id(&self, conn: &AsyncPgConnection) -> AppResult<i32> {
        load_crate_id(conn, &self.name).await
    }
}

pub async fn load_crate(mut conn: &AsyncPgConnection, name: &str) -> AppResult<Crate> {
    Crate::by_name(name)
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(name))
}

pub async fn load_crate_id(mut conn: &AsyncPgConnection, name: &str) -> AppResult<i32> {
    Crate::by_name(name)
        .select(crates::id)
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(name))
}
