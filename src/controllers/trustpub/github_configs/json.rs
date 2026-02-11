use axum::Json;
use axum::extract::FromRequest;
use serde::{Deserialize, Serialize};

pub use crate::views::trustpub::{GitHubConfig, NewGitHubConfig};

#[derive(Debug, Deserialize, FromRequest, utoipa::ToSchema)]
#[from_request(via(Json))]
pub struct CreateRequest {
    #[schema(inline)]
    pub github_config: NewGitHubConfig,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateResponse {
    pub github_config: GitHubConfig,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListResponse {
    pub github_configs: Vec<GitHubConfig>,

    #[schema(inline)]
    pub meta: ListResponseMeta,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListResponseMeta {
    /// The total number of GitHub configs belonging to the crate.
    #[schema(example = 42)]
    pub total: i64,

    /// Query string to the next page of results, if any.
    #[schema(example = "?seek=abc123")]
    pub next_page: Option<String>,
}
