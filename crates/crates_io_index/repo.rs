use crate::credentials::Credentials;
use anyhow::{Context, anyhow};
use base64::{Engine, engine::general_purpose};
use crates_io_env_vars::{required_var, required_var_parsed, var};
use secrecy::{ExposeSecret, SecretString};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tempfile::TempDir;
use url::Url;

pub struct RepositoryConfig {
    pub index_location: Url,
    pub credentials: Credentials,
}

impl RepositoryConfig {
    pub fn from_environment() -> anyhow::Result<Self> {
        let repo_url: Url = required_var_parsed("GIT_REPO_URL")?;
        let is_ssh = repo_url.scheme() == "ssh";

        let username = var("GIT_HTTP_USER")?;
        let password = var("GIT_HTTP_PWD")?.map(SecretString::from);

        match (is_ssh, username, password) {
            (true, username, password) => {
                let ssh_key = SecretString::from(required_var("GIT_SSH_KEY")?);

                if username.is_some() || password.is_some() {
                    warn!("both http and ssh credentials to authenticate with git are set");
                    info!("note: ssh credentials will take precedence over the http ones");
                }

                let key = general_purpose::STANDARD
                    .decode(ssh_key.expose_secret())
                    .expect("failed to base64 decode the ssh key");
                let key =
                    String::from_utf8(key).expect("failed to convert the ssh key to a string");
                let credentials = Credentials::Ssh { key: key.into() };

                Ok(Self {
                    index_location: repo_url,
                    credentials,
                })
            }
            (false, Some(username), Some(password)) => {
                let credentials = Credentials::Http { username, password };

                Ok(Self {
                    index_location: repo_url,
                    credentials,
                })
            }

            (false, _, _) => Ok(Self {
                index_location: repo_url,
                credentials: Credentials::Missing,
            }),
        }
    }
}

pub struct Repository {
    checkout_path: TempDir,
    repository: git2::Repository,
    credentials: Credentials,
}

impl Repository {
    /// Clones the crate index from a remote git server and returns a
    /// `Repository` struct to interact with the local copy of the crate index.
    ///
    /// Note that the `user` configuration for the repository is automatically
    /// set to `bors <bors@rust-lang.org>`.
    ///
    /// # Errors
    ///
    /// - If creation of a temporary folder for cloning the crate index fails.
    /// - If cloning the crate index fails.
    /// - If reading the global git config fails.
    ///
    #[instrument(skip_all)]
    pub fn open(repository_config: &RepositoryConfig) -> anyhow::Result<Self> {
        let checkout_path = tempfile::Builder::new()
            .prefix("git")
            .tempdir()
            .context("Failed to create temporary directory")?;

        let Some(checkout_path_str) = checkout_path.path().to_str() else {
            return Err(anyhow!("Failed to convert Path to &str"));
        };

        run_via_cli(
            Command::new("git").args([
                "clone",
                "--single-branch",
                repository_config.index_location.as_str(),
                checkout_path_str,
            ]),
            &repository_config.credentials,
        )
        .context("Failed to clone index repository")?;

        let repository = git2::Repository::open(checkout_path.path())
            .context("Failed to open cloned index repository")?;

        // All commits to the index registry made through crates.io will be made by bors, the Rust
        // community's friendly GitHub bot.

        let mut cfg = repository
            .config()
            .context("Failed to read git configuration")?;

        cfg.set_str("user.name", "bors")
            .context("Failed to set user name")?;

        cfg.set_str("user.email", "bors@rust-lang.org")
            .context("Failed to set user email address")?;

        Ok(Self {
            checkout_path,
            repository,
            credentials: repository_config.credentials.clone(),
        })
    }

    /// Returns the absolute path to the crate index file that corresponds to
    /// the given crate name.
    ///
    /// This is similar to [Self::relative_index_file], but returns the absolute
    /// path.
    pub fn index_file(&self, name: &str) -> PathBuf {
        self.checkout_path
            .path()
            .join(Self::relative_index_file(name))
    }

    /// Returns the relative path to the crate index file.
    /// Does not perform conversion to lowercase.
    fn relative_index_file_helper(name: &str) -> Vec<&str> {
        match name.len() {
            1 => vec!["1", name],
            2 => vec!["2", name],
            3 => vec!["3", &name[..1], name],
            _ => vec![&name[0..2], &name[2..4], name],
        }
    }

    /// Returns the relative path to the crate index file that corresponds to
    /// the given crate name as a path (i.e. with platform-dependent folder separators).
    ///
    /// see <https://doc.rust-lang.org/cargo/reference/registries.html#index-format>
    pub fn relative_index_file(name: &str) -> PathBuf {
        let name = name.to_lowercase();
        Self::relative_index_file_helper(&name).iter().collect()
    }

    /// Returns the relative path to the crate index file that corresponds to
    /// the given crate name for usage in URLs (i.e. with `/` separator).
    ///
    /// see <https://doc.rust-lang.org/cargo/reference/registries.html#index-format>
    pub fn relative_index_file_for_url(name: &str) -> String {
        let name = name.to_lowercase();
        Self::relative_index_file_helper(&name).join("/")
    }

    /// Returns the [Object ID](git2::Oid) of the currently checked out commit
    /// in the local crate index repository.
    ///
    /// # Errors
    ///
    /// - If the `HEAD` pointer can't be retrieved.
    ///
    pub fn head_oid(&self) -> anyhow::Result<git2::Oid> {
        let repo = &self.repository;
        let head = repo.head().context("Failed to read HEAD reference")?;
        Ok(head.target().unwrap())
    }

    /// Commits the specified files with the specified commit message and pushes
    /// the commit to the `master` branch on the `origin` remote.
    ///
    /// Note that `modified_files` expects file paths **relative** to the
    /// repository working folder!
    #[instrument(skip_all, fields(message = %msg, num_files = modified_files.len()))]
    fn perform_commit_and_push(&self, msg: &str, modified_files: &[&Path]) -> anyhow::Result<()> {
        let mut index = self.repository.index()?;

        for modified_file in modified_files {
            if self.checkout_path.path().join(modified_file).exists() {
                index.add_path(modified_file)?;
            } else {
                index.remove_path(modified_file)?;
            }
        }

        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = self.repository.find_tree(tree_id)?;

        // git commit -m "..."
        let head = self.head_oid()?;
        let parent = self.repository.find_commit(head)?;
        let sig = self.repository.signature()?;
        self.repository
            .commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&parent])?;

        self.push()
    }

    /// Gets a list of files that have been modified since a given `starting_commit`
    /// (use `starting_commit = None` for a list of all files).
    #[instrument(skip_all)]
    pub fn get_files_modified_since(
        &self,
        starting_commit: Option<&str>,
    ) -> anyhow::Result<Vec<PathBuf>> {
        let starting_commit = match starting_commit {
            Some(starting_commit) => {
                let oid = git2::Oid::from_str(starting_commit)
                    .context("failed to parse commit into Oid")?;
                let commit = self
                    .repository
                    .find_commit(oid)
                    .context("failed to find commit")?;
                Some(
                    commit
                        .as_object()
                        .peel_to_tree()
                        .context("failed to find tree for commit")?,
                )
            }
            None => None,
        };

        let head = self
            .repository
            .find_commit(self.head_oid()?)?
            .as_object()
            .peel_to_tree()
            .context("failed to find tree for HEAD")?;
        let diff = self
            .repository
            .diff_tree_to_tree(starting_commit.as_ref(), Some(&head), None)
            .context("failed to run diff")?;
        let files = diff
            .deltas()
            .map(|delta| delta.new_file())
            .filter(|file| file.exists())
            .map(|file| file.path().unwrap().to_path_buf())
            .collect();

        Ok(files)
    }

    /// Push the current branch to the provided refname
    #[instrument(skip_all)]
    fn push(&self) -> anyhow::Result<()> {
        self.run_command(Command::new("git").args(["push", "origin", "HEAD:master"]))
    }

    /// Commits the specified files with the specified commit message and pushes
    /// the commit to the `master` branch on the `origin` remote.
    ///
    /// Note that `modified_files` expects **absolute** file paths!
    ///
    /// This function also prints the commit message and a success or failure
    /// message to the console.
    pub fn commit_and_push(&self, message: &str, modified_files: &[&Path]) -> anyhow::Result<()> {
        info!("Committing and pushing \"{message}\"");

        let checkout_path = self.checkout_path.path();
        let relative_paths: Vec<&Path> = modified_files
            .iter()
            .map(|p| p.strip_prefix(checkout_path))
            .collect::<Result<_, _>>()?;

        self.perform_commit_and_push(message, &relative_paths)
            .map(|_| info!("Commit and push finished for \"{message}\""))
            .map_err(|err| {
                error!(?err, "Commit and push for \"{message}\" errored");
                err
            })
    }

    /// Fetches any changes from the `origin` remote and performs a hard reset
    /// to the tip of the `origin/master` branch.
    #[instrument(skip_all)]
    pub fn reset_head(&self) -> anyhow::Result<()> {
        let original_head = self.head_oid()?;

        let fetch_start = Instant::now();
        self.run_command(Command::new("git").args(["fetch", "origin", "master"]))?;
        info!(duration = fetch_start.elapsed().as_nanos(), "Index fetched");

        let reset_start = Instant::now();
        self.run_command(Command::new("git").args(["reset", "--hard", "origin/master"]))?;
        info!(duration = reset_start.elapsed().as_nanos(), "Index reset");

        let head = self.head_oid()?;
        if head != original_head {
            // Ensure that the internal state of `self.repository` is updated correctly
            self.repository.checkout_head(None)?;

            info!("Index reset from {original_head} to {head}");
        }

        Ok(())
    }

    /// Reset `HEAD` to a single commit with all the index contents, but no parent
    #[instrument(skip_all)]
    pub fn squash_to_single_commit(&self, msg: &str) -> anyhow::Result<()> {
        let tree = self.repository.find_commit(self.head_oid()?)?.tree()?;
        let sig = self.repository.signature()?;

        // We cannot update an existing `update_ref`, because that requires the
        // first parent of this commit to match the ref's current value.
        // Instead, create the commit and then do a hard reset.
        let commit = self.repository.commit(None, &sig, &sig, msg, &tree, &[])?;
        let commit = self
            .repository
            .find_object(commit, Some(git2::ObjectType::Commit))?;
        self.repository
            .reset(&commit, git2::ResetType::Hard, None)?;

        Ok(())
    }

    /// Runs the specified `git` command in the working directory of the local
    /// crate index repository.
    ///
    /// This function also temporarily sets the `GIT_SSH_COMMAND` environment
    /// variable to ensure that `git push` commands are able to succeed.
    pub fn run_command(&self, command: &mut Command) -> anyhow::Result<()> {
        let checkout_path = self.checkout_path.path();
        command.current_dir(checkout_path);

        run_via_cli(command, &self.credentials)
    }
}

/// Runs the specified `git` command through the `git` CLI.
///
/// This function also temporarily sets the `GIT_SSH_COMMAND` environment
/// variable to ensure that `git push` commands are able to succeed.
#[instrument(skip_all)]
pub fn run_via_cli(command: &mut Command, credentials: &Credentials) -> anyhow::Result<()> {
    let temp_key_path = credentials
        .ssh_key()
        .map(|_| credentials.write_temporary_ssh_key())
        .transpose()?;

    if let Some(temp_key_path) = &temp_key_path {
        command.env(
            "GIT_SSH_COMMAND",
            format!("ssh -i {}", temp_key_path.display()),
        );
    }

    debug!(?command);
    let output = command.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow!(
            "Running git command failed with: {}{}",
            stderr,
            stdout
        ));
    }

    Ok(())
}
