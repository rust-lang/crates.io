use anyhow::Context;
use crates_io_env_vars::{required_var, var};
use secrecy::SecretString;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum CdnLogStorageConfig {
    S3 {
        access_key: String,
        secret_key: SecretString,
    },
    Local {
        path: PathBuf,
    },
    Memory,
}

impl CdnLogStorageConfig {
    pub fn s3(access_key: String, secret_key: SecretString) -> Self {
        Self::S3 {
            access_key,
            secret_key,
        }
    }

    pub fn local(path: PathBuf) -> Self {
        Self::Local { path }
    }

    pub fn memory() -> Self {
        Self::Memory
    }

    pub fn from_env() -> anyhow::Result<Self> {
        if let Some(access_key) = var("AWS_ACCESS_KEY")? {
            let secret_key = required_var("AWS_SECRET_KEY")?.into();
            return Ok(Self::s3(access_key, secret_key));
        }

        let current_dir = std::env::current_dir();
        let current_dir = current_dir.context("Failed to read the current directory")?;

        let path = current_dir.join("local_uploads");
        let path_display = path.display();
        if path.exists() {
            info!("Falling back to local CDN log storage at {path_display}");
            return Ok(Self::local(path));
        }

        warn!("Falling back to in-memory CDN log storage because {path_display} does not exist");
        Ok(Self::memory())
    }
}
