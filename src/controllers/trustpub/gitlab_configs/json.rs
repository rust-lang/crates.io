use axum::Json;
use axum::extract::FromRequest;
use serde::{Deserialize, Serialize};

pub use crate::views::trustpub::{GitLabConfig, NewGitLabConfig};

#[derive(Debug, Deserialize, FromRequest, utoipa::ToSchema)]
#[from_request(via(Json))]
pub struct CreateRequest {
    #[schema(inline)]
    pub gitlab_config: NewGitLabConfig,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateResponse {
    pub gitlab_config: GitLabConfig,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListResponse {
    pub gitlab_configs: Vec<GitLabConfig>,

    #[schema(inline)]
    pub meta: ListResponseMeta,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListResponseMeta {
    /// The total number of GitLab configs belonging to the crate.
    #[schema(example = 42)]
    pub total: i64,

    /// Query string to the next page of results, if any.
    #[schema(example = "?seek=abc123")]
    pub next_page: Option<String>,
}
