use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[schema(as = GitHubConfig)]
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
#[schema(as = NewGitHubConfig)]
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[schema(as = GitLabConfig)]
pub struct GitLabConfig {
    #[schema(example = 42)]
    pub id: i32,
    #[schema(example = "regex")]
    #[serde(rename = "crate")]
    pub krate: String,
    #[schema(example = "rust-lang")]
    pub namespace: String,
    #[schema(example = json!(null))]
    pub namespace_id: Option<String>,
    #[schema(example = "regex")]
    pub project: String,
    #[schema(example = ".gitlab-ci.yml")]
    pub workflow_filepath: String,
    #[schema(example = json!(null))]
    pub environment: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = NewGitLabConfig)]
pub struct NewGitLabConfig {
    #[schema(example = "regex")]
    #[serde(rename = "crate")]
    pub krate: String,
    #[schema(example = "rust-lang")]
    pub namespace: String,
    #[schema(example = "regex")]
    pub project: String,
    #[schema(example = ".gitlab-ci.yml")]
    pub workflow_filepath: String,
    #[schema(example = json!(null))]
    pub environment: Option<String>,
}
