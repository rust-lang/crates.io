use axum::Json;
use axum::extract::FromRequest;
use serde::{Deserialize, Serialize};

pub use crate::views::trustpub::{GitLabConfig, NewGitLabConfig};

#[derive(Debug, Deserialize, FromRequest, utoipa::ToSchema)]
#[from_request(via(Json))]
pub struct CreateRequest {
    pub gitlab_config: NewGitLabConfig,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateResponse {
    pub gitlab_config: GitLabConfig,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListResponse {
    pub gitlab_configs: Vec<GitLabConfig>,
}
