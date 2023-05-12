use anyhow::{anyhow, Context};
use std::io::Write;

static DEFAULT_GIT_SSH_USERNAME: &str = "git";

#[derive(Clone)]
pub enum Credentials {
    Missing,
    Http { username: String, password: String },
    Ssh { key: String },
}

impl Credentials {
    pub(crate) fn ssh_key(&self) -> Option<&str> {
        match self {
            Credentials::Ssh { key } => Some(key),
            _ => None,
        }
    }

    pub(crate) fn git2_callback(
        &self,
        user_from_url: Option<&str>,
        cred_type: git2::CredentialType,
    ) -> Result<git2::Cred, git2::Error> {
        match self {
            Credentials::Missing => Err(git2::Error::from_str("no authentication set")),
            Credentials::Http { username, password } => {
                git2::Cred::userpass_plaintext(username, password)
            }
            Credentials::Ssh { key } => {
                // git2 might call the callback two times when requesting credentials:
                //
                // 1. If the username is not specified in the URL, the first call will request it,
                //    without asking for the SSH key.
                //
                // 2. The other call will request the proper SSH key, and the username must be the
                //    same one either specified in the URL or the previous call.
                //
                // More information on this behavior is available at the following links:
                // - https://github.com/rust-lang/git2-rs/issues/329
                // - https://libgit2.org/docs/guides/authentication/
                let user = user_from_url.unwrap_or(DEFAULT_GIT_SSH_USERNAME);
                if cred_type.contains(git2::CredentialType::USERNAME) {
                    git2::Cred::username(user)
                } else {
                    git2::Cred::ssh_key_from_memory(user, None, key, None)
                }
            }
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
            .write_all(key.as_bytes())
            .context("Failed to write SSH key to temporary file")?;

        Ok(temp_key_file.into_temp_path())
    }
}
