use std::path::PathBuf;
use {Uploader, Replica};

#[derive(Clone, Debug)]
pub struct Config {
    pub uploader: Uploader,
    pub session_key: String,
    pub git_repo_checkout: PathBuf,
    pub gh_client_id: String,
    pub gh_client_secret: String,
    pub db_url: String,
    pub env: ::Env,
    pub max_upload_size: u64,
    pub mirror: Replica,
    pub api_protocol: String,
}
