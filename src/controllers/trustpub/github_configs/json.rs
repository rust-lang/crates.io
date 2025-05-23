use axum::Json;
use axum::extract::FromRequest;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GitHubConfig {
    #[schema(example = 42)]
    pub id: i32,
    #[schema(example = "regex")]
    #[serde(rename = "crate")]
    pub krate: String,
    #[schema(example = "rust-lang")]
    pub repository_owner: String,
    #[schema(example = 5430905)]
    pub repository_owner_id: i32,
    #[schema(example = "regex")]
    pub repository_name: String,
    #[schema(example = "ci.yml")]
    pub workflow_filename: String,
    #[schema(example = json!(null))]
    pub environment: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct NewGitHubConfig {
    #[schema(example = "regex")]
    #[serde(rename = "crate")]
    pub krate: String,
    #[schema(example = "rust-lang")]
    pub repository_owner: String,
    #[schema(example = "regex")]
    pub repository_name: String,
    #[schema(example = "ci.yml")]
    pub workflow_filename: String,
    #[schema(example = json!(null))]
    pub environment: Option<String>,
}

#[derive(Debug, Deserialize, FromRequest, utoipa::ToSchema)]
#[from_request(via(Json))]
pub struct CreateRequest {
    pub github_config: NewGitHubConfig,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateResponse {
    pub github_config: GitHubConfig,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListResponse {
    pub github_configs: Vec<GitHubConfig>,
}
