use crate::commit_builder::CommitBuilder;
use crate::credentials::Credentials;
use anyhow::{Context, anyhow};
use base64::{Engine, engine::general_purpose};
use crates_io_env_vars::{required_var, required_var_parsed, var};
use secrecy::{ExposeSecret, SecretString};
use std::path::PathBuf;
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
                "--bare",
                "--single-branch",
                repository_config.index_location.as_str(),
                checkout_path_str,
            ]),
            &repository_config.credentials,
        )
        .context("Failed to clone index repository")?;

        let repository = git2::Repository::open_bare(checkout_path.path())
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

    /// Starts a new commit targeting the `master` branch.
    ///
    /// See [`Self::commit_builder_to`] for details.
    pub fn commit_builder(&self, msg: impl Into<String>) -> anyhow::Result<CommitBuilder<'_>> {
        CommitBuilder::new(self, msg, "master")
    }

    /// Starts a new commit targeting the given remote branch.
    ///
    /// Stage changes on the returned [`CommitBuilder`] and call
    /// [`CommitBuilder::commit_and_push`] to finalize them.
    pub fn commit_builder_to(
        &self,
        msg: impl Into<String>,
        branch: impl Into<String>,
    ) -> anyhow::Result<CommitBuilder<'_>> {
        CommitBuilder::new(self, msg, branch)
    }

    pub(crate) fn git_repo(&self) -> &git2::Repository {
        &self.repository
    }

    /// Returns the crate names of all entries currently stored in the index.
    ///
    /// Top-level files (e.g. `config.json`) and the top-level `.github`
    /// folder are excluded; only blobs nested under the sharded
    /// `N[/prefix]/name` layout are returned.
    pub fn list_entries(&self) -> anyhow::Result<Vec<String>> {
        let tree = self
            .repository
            .head()
            .context("Failed to read HEAD reference")?
            .peel_to_tree()
            .context("Failed to find tree for HEAD")?;

        let mut names = Vec::new();
        tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
            // Skip the top-level `.github` folder (GitHub Actions workflows, etc.).
            if root.is_empty() && entry.name() == Some(".github") {
                return git2::TreeWalkResult::Skip;
            }

            if !root.is_empty()
                && entry.kind() == Some(git2::ObjectType::Blob)
                && let Some(name) = entry.name()
            {
                names.push(name.to_string());
            }
            git2::TreeWalkResult::Ok
        })
        .context("Failed to walk HEAD tree")?;

        Ok(names)
    }

    /// Reads the contents of the index entry for the given crate name.
    ///
    /// Returns `Ok(None)` if no entry exists for the crate.
    pub fn read_entry(&self, name: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let tree = self
            .repository
            .head()
            .context("Failed to read HEAD reference")?
            .peel_to_tree()
            .context("Failed to find tree for HEAD")?;

        let path = Self::relative_index_file(name);
        match tree.get_path(&path) {
            Ok(entry) => {
                let blob = entry
                    .to_object(&self.repository)
                    .context("Failed to resolve tree entry")?
                    .peel_to_blob()
                    .context("Failed to peel tree entry to blob")?;
                Ok(Some(blob.content().to_vec()))
            }
            Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
            Err(error) => {
                Err(error).with_context(|| format!("Failed to look up tree entry for `{name}`"))
            }
        }
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

    /// Fetches any changes from the `origin` remote and force-updates the
    /// local `refs/heads/master` ref to the fetched tip.
    #[instrument(skip_all)]
    pub fn reset_head(&self) -> anyhow::Result<()> {
        let original_head = self.head_oid()?;

        let fetch_start = Instant::now();
        self.run_command(Command::new("git").args(["fetch", "origin", "master"]))?;
        info!(duration = fetch_start.elapsed().as_nanos(), "Index fetched");

        let fetch_head = self
            .repository
            .refname_to_id("FETCH_HEAD")
            .context("Failed to resolve FETCH_HEAD")?;
        self.repository
            .reference("refs/heads/master", fetch_head, true, "reset_head")
            .context("Failed to update refs/heads/master")?;

        let head = self.head_oid()?;
        if head != original_head {
            info!("Index reset from {original_head} to {head}");
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::UpstreamIndex;
    use claims::{assert_err, assert_none, assert_ok_eq, assert_some_eq};

    fn setup() -> (UpstreamIndex, Repository) {
        let upstream = UpstreamIndex::new().unwrap();
        let config = RepositoryConfig {
            index_location: upstream.url(),
            credentials: Credentials::Missing,
        };
        let repo = Repository::open(&config).unwrap();
        (upstream, repo)
    }

    #[test]
    fn read_entry_missing() {
        let (_upstream, repo) = setup();
        assert_ok_eq!(repo.read_entry("serde"), None::<Vec<u8>>);
    }

    #[test]
    fn read_entry_present() {
        let (upstream, repo) = setup();
        upstream.write_file("se/rd/serde", "hello\n").unwrap();
        repo.reset_head().unwrap();

        let entry = repo.read_entry("serde").unwrap();
        assert_some_eq!(entry, b"hello\n".to_vec());
    }

    #[test]
    fn read_entry_error_mentions_name() {
        let (_upstream, repo) = setup();

        // A null byte in the crate name forces `git2` to fail the path
        // conversion with a non-`NotFound` error, exercising the error
        // context branch of `read_entry()`.
        let err = assert_err!(repo.read_entry("\0serde"));
        insta::assert_snapshot!(err, @"Failed to look up tree entry for `\0serde`");
    }

    #[test]
    fn read_entry_ignores_top_level_files() {
        let (upstream, repo) = setup();
        upstream.write_file("config.json", "{}").unwrap();
        repo.reset_head().unwrap();

        assert_none!(repo.read_entry("config.json").unwrap());
    }

    #[test]
    fn list_entries_empty() {
        let (_upstream, repo) = setup();
        assert_ok_eq!(repo.list_entries(), Vec::<String>::new());
    }

    #[test]
    fn list_entries_returns_crate_names() {
        let (upstream, repo) = setup();
        upstream.write_file("1/a", "").unwrap();
        upstream.write_file("2/ab", "").unwrap();
        upstream.write_file("3/a/abc", "").unwrap();
        upstream.write_file("se/rd/serde", "").unwrap();
        repo.reset_head().unwrap();

        let mut entries = repo.list_entries().unwrap();
        entries.sort();
        assert_eq!(entries, vec!["a", "ab", "abc", "serde"]);
    }

    #[test]
    fn list_entries_excludes_top_level_files() {
        let (upstream, repo) = setup();
        upstream.write_file("config.json", "{}").unwrap();
        upstream.write_file("se/rd/serde", "").unwrap();
        repo.reset_head().unwrap();

        assert_ok_eq!(repo.list_entries(), vec!["serde".to_string()]);
    }

    #[test]
    fn list_entries_excludes_github_folder() {
        let (upstream, repo) = setup();
        upstream
            .write_file(".github/workflows/ci.yml", "name: CI\n")
            .unwrap();
        upstream.write_file("se/rd/serde", "").unwrap();
        repo.reset_head().unwrap();

        assert_ok_eq!(repo.list_entries(), vec!["serde".to_string()]);
    }
}
