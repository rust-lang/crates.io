use anyhow::{Context, anyhow};
use secrecy::{ExposeSecret, SecretString};
use std::io::Write;

#[derive(Clone)]
pub enum Credentials {
    Missing,
    Http {
        username: String,
        password: SecretString,
    },
    Ssh {
        key: SecretString,
    },
}

impl Credentials {
    pub(crate) fn ssh_key(&self) -> Option<&SecretString> {
        match self {
            Credentials::Ssh { key } => Some(key),
            _ => None,
        }
    }

    /// Write the SSH key to a temporary file and return the path. The file is
    /// deleted once the returned path is dropped.
    ///
    /// This function can be used when running `git push` instead of using the
    /// `git2` crate for pushing commits to remote git servers.
    ///
    /// Note: On Linux this function creates the temporary file in `/dev/shm` to
    /// avoid writing it to disk.
    ///
    /// # Errors
    ///
    /// - If non-SSH credentials are use, `Err` is returned.
    /// - If creation of the temporary file fails, `Err` is returned.
    ///
    pub(crate) fn write_temporary_ssh_key(&self) -> anyhow::Result<tempfile::TempPath> {
        let key = self
            .ssh_key()
            .ok_or_else(|| anyhow!("SSH key not available"))?;

        let dir = if cfg!(target_os = "linux") {
            // When running on production, ensure the file is created in tmpfs and not persisted to disk
            "/dev/shm".into()
        } else {
            // For other platforms, default to std::env::tempdir()
            std::env::temp_dir()
        };

        let mut temp_key_file = tempfile::Builder::new()
            .tempfile_in(dir)
            .context("Failed to create temporary file")?;

        temp_key_file
            .write_all(key.expose_secret().as_bytes())
            .context("Failed to write SSH key to temporary file")?;

        Ok(temp_key_file.into_temp_path())
    }
}
