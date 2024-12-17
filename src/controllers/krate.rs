use crate::models::Crate;
use crate::util::errors::{crate_not_found, AppResult};
use axum::extract::{FromRequestParts, Path};
use crates_io_database::schema::crates;
use diesel::{OptionalExtension, QueryDsl};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use utoipa::IntoParams;

pub mod delete;
pub mod downloads;
pub mod follow;
pub mod metadata;
pub mod owners;
pub mod publish;
pub mod rev_deps;
pub mod search;
pub mod versions;

#[derive(Deserialize, FromRequestParts, IntoParams)]
#[into_params(parameter_in = Path)]
#[from_request(via(Path))]
pub struct CratePath {
    /// Name of the crate
    pub name: String,
}

impl CratePath {
    pub async fn load_crate(&self, conn: &mut AsyncPgConnection) -> AppResult<Crate> {
        load_crate(conn, &self.name).await
    }

    pub async fn load_crate_id(&self, conn: &mut AsyncPgConnection) -> AppResult<i32> {
        load_crate_id(conn, &self.name).await
    }
}

pub async fn load_crate(conn: &mut AsyncPgConnection, name: &str) -> AppResult<Crate> {
    Crate::by_name(name)
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(name))
}

pub async fn load_crate_id(conn: &mut AsyncPgConnection, name: &str) -> AppResult<i32> {
    Crate::by_name(name)
        .select(crates::id)
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(name))
}
