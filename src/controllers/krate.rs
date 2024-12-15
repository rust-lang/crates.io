use axum::extract::{FromRequestParts, Path};

pub mod delete;
pub mod downloads;
pub mod follow;
pub mod metadata;
pub mod owners;
pub mod publish;
pub mod search;
pub mod versions;

#[derive(Deserialize, FromRequestParts)]
#[from_request(via(Path))]
pub struct CratePath {
    /// Name of the crate
    pub name: String,
}
