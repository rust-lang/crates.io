#[macro_use]
extern crate serde;
#[macro_use]
extern crate tracing;

#[cfg(feature = "testing")]
pub mod testing;

use anyhow::{anyhow, Context};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;
use url::Url;

static DEFAULT_GIT_SSH_USERNAME: &str = "git";

#[derive(Clone)]
pub enum Credentials {
    Missing,
    Http { username: String, password: String },
    Ssh { key: String },
}

impl Credentials {
    fn ssh_key(&self) -> Option<&str> {
        match self {
            Credentials::Ssh { key } => Some(key),
            _ => None,
        }
    }

    fn git2_callback(
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
    fn write_temporary_ssh_key(&self) -> anyhow::Result<tempfile::TempPath> {
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Crate {
    pub name: String,
    pub vers: String,
    pub deps: Vec<Dependency>,
    pub cksum: String,
    pub features: BTreeMap<String, Vec<String>>,
    /// This field contains features with new, extended syntax. Specifically,
    /// namespaced features (`dep:`) and weak dependencies (`pkg?/feat`).
    ///
    /// It is only populated if a feature uses the new syntax. Cargo merges it
    /// on top of the `features` field when reading the entries.
    ///
    /// This is separated from `features` because versions older than 1.19
    /// will fail to load due to not being able to parse the new syntax, even
    /// with a `Cargo.lock` file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features2: Option<BTreeMap<String, Vec<String>>>,
    pub yanked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_version: Option<String>,
    /// The schema version for this entry.
    ///
    /// If this is None, it defaults to version 1. Entries with unknown
    /// versions are ignored by cargo starting with 1.51.
    ///
    /// Version `2` format adds the `features2` field.
    ///
    /// This provides a method to safely introduce changes to index entries
    /// and allow older versions of cargo to ignore newer entries it doesn't
    /// understand. This is honored as of 1.51, so unfortunately older
    /// versions will ignore it, and potentially misinterpret version 2 and
    /// newer entries.
    ///
    /// The intent is that versions older than 1.51 will work with a
    /// pre-existing `Cargo.lock`, but they may not correctly process `cargo
    /// update` or build a lock from scratch. In that case, cargo may
    /// incorrectly select a new package that uses a new index format. A
    /// workaround is to downgrade any packages that are incompatible with the
    /// `--precise` flag of `cargo update`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v: Option<u32>,
}

impl Crate {
    pub fn write_to<W: Write>(&self, mut writer: W) -> anyhow::Result<()> {
        serde_json::to_writer(&mut writer, self)?;
        writer.write_all(b"\n")?;
        Ok(())
    }
}

pub fn write_crates<W: Write>(crates: &[Crate], mut writer: W) -> anyhow::Result<()> {
    for krate in crates {
        krate.write_to(&mut writer)?;
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: Option<DependencyKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
}

impl PartialOrd for Dependency {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Dependency {
    fn cmp(&self, other: &Self) -> Ordering {
        // In old `cargo` versions the dependency order appears to matter if the
        // same dependency exists twice but with different `kind` fields. In
        // those cases the `optional` field can sometimes be ignored or
        // misinterpreted. With this manual `Ord` implementation we ensure that
        // `normal` dependencies are always first when multiple with the same
        // `name` exist.
        (
            &self.name,
            self.kind,
            &self.req,
            self.optional,
            self.default_features,
            &self.target,
            &self.package,
            &self.features,
        )
            .cmp(&(
                &other.name,
                other.kind,
                &other.req,
                other.optional,
                other.default_features,
                &other.target,
                &other.package,
                &other.features,
            ))
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, PartialOrd, Ord, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Normal,
    Build,
    Dev,
}

pub struct RepositoryConfig {
    pub index_location: Url,
    pub credentials: Credentials,
}

impl RepositoryConfig {
    pub fn from_environment() -> Self {
        let username = dotenvy::var("GIT_HTTP_USER");
        let password = dotenvy::var("GIT_HTTP_PWD");
        let http_url = dotenvy::var("GIT_REPO_URL");

        let ssh_key = dotenvy::var("GIT_SSH_KEY");
        let ssh_url = dotenvy::var("GIT_SSH_REPO_URL");

        match (username, password, http_url, ssh_key, ssh_url) {
            (extra_user, extra_pass, extra_http_url, Ok(encoded_key), Ok(ssh_url)) => {
                if let (Ok(_), Ok(_), Ok(_)) = (extra_user, extra_pass, extra_http_url) {
                    warn!("both http and ssh credentials to authenticate with git are set");
                    info!("note: ssh credentials will take precedence over the http ones");
                }

                let index_location =
                    Url::parse(&ssh_url).expect("failed to parse GIT_SSH_REPO_URL");

                let credentials = Credentials::Ssh {
                    key: String::from_utf8(
                        base64::decode(encoded_key).expect("failed to base64 decode the ssh key"),
                    )
                    .expect("failed to convert the ssh key to a string"),
                };

                Self {
                    index_location,
                    credentials,
                }
            }
            (Ok(username), Ok(password), Ok(http_url), Err(_), Err(_)) => {
                let index_location = Url::parse(&http_url).expect("failed to parse GIT_REPO_URL");
                let credentials = Credentials::Http { username, password };

                Self {
                    index_location,
                    credentials,
                }
            }
            (_, _, Ok(http_url), _, _) => {
                let index_location = Url::parse(&http_url).expect("failed to parse GIT_REPO_URL");
                let credentials = Credentials::Missing;

                Self {
                    index_location,
                    credentials,
                }
            }
            _ => panic!("must have `GIT_REPO_URL` defined"),
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

    /// Commits the specified file with the specified commit message and pushes
    /// the commit to the `master` branch on the `origin` remote.
    ///
    /// Note that `modified_file` expects a file path **relative** to the
    /// repository working folder!
    #[instrument(skip_all, fields(message = %msg))]
    fn perform_commit_and_push(&self, msg: &str, modified_file: &Path) -> anyhow::Result<()> {
        // git add $file
        let mut index = self.repository.index()?;
        index.add_path(modified_file)?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = self.repository.find_tree(tree_id)?;

        // git commit -m "..."
        let head = self.head_oid()?;
        let parent = self.repository.find_commit(head)?;
        let sig = self.repository.signature()?;
        self.repository
            .commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&parent])?;

        self.push("refs/heads/master")
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
    fn push(&self, refspec: &str) -> anyhow::Result<()> {
        let mut ref_status = Ok(());
        let mut callback_called = false;
        {
            let mut origin = self.repository.find_remote("origin")?;
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.credentials(|_, user_from_url, cred_type| {
                self.credentials.git2_callback(user_from_url, cred_type)
            });
            callbacks.push_update_reference(|_, status| {
                if let Some(s) = status {
                    ref_status = Err(anyhow!("failed to push a ref: {}", s))
                }
                callback_called = true;
                Ok(())
            });
            let mut opts = git2::PushOptions::new();
            opts.remote_callbacks(callbacks);
            origin.push(&[refspec], Some(&mut opts))?;
        }

        if !callback_called {
            ref_status = Err(anyhow!("update_reference callback was not called"));
        }

        ref_status
    }

    /// Commits the specified file with the specified commit message and pushes
    /// the commit to the `master` branch on the `origin` remote.
    ///
    /// Note that `modified_file` expects an **absolute** file path!
    ///
    /// This function also prints the commit message and a success or failure
    /// message to the console.
    pub fn commit_and_push(&self, message: &str, modified_file: &Path) -> anyhow::Result<()> {
        info!("Committing and pushing \"{message}\"");

        let relative_path = modified_file.strip_prefix(self.checkout_path.path())?;
        self.perform_commit_and_push(message, relative_path)
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
        let mut origin = self.repository.find_remote("origin")?;
        let original_head = self.head_oid()?;
        origin.fetch(
            // Force overwrite (`+` prefix) local master branch with the server's master branch.
            // The git CLI will refuse to fetch into the current branch of a non-bare repo
            // but libgit2 doesn't seem to prevent this potential footgun.
            // The entire point is to do a hard reset, so this footgun is not a concern.
            &["+refs/heads/master:refs/heads/master"],
            Some(&mut Self::fetch_options(&self.credentials)),
            None,
        )?;
        let head = self.head_oid()?;

        if head != original_head {
            info!("Resetting index from {original_head} to {head}");
        }

        let obj = self.repository.find_object(head, None)?;
        self.repository.reset(&obj, git2::ResetType::Hard, None)?;
        Ok(())
    }

    fn fetch_options(credentials: &Credentials) -> git2::FetchOptions<'_> {
        let mut callbacks = git2::RemoteCallbacks::new();

        callbacks.credentials(move |_, user_from_url, cred_type| {
            credentials.git2_callback(user_from_url, cred_type)
        });

        let mut opts = git2::FetchOptions::new();
        opts.remote_callbacks(callbacks);
        opts
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

#[cfg(test)]
mod tests {
    use super::*;
    use claims::*;

    #[test]
    fn crate_writer() {
        let krate = Crate {
            name: "foo".to_string(),
            vers: "1.2.3".to_string(),
            deps: vec![],
            cksum: "0123456789asbcdef".to_string(),
            features: Default::default(),
            features2: None,
            yanked: None,
            links: None,
            rust_version: None,
            v: None,
        };
        let mut buffer = Vec::new();
        assert_ok!(krate.write_to(&mut buffer));
        assert_ok_eq!(String::from_utf8(buffer), "\
            {\"name\":\"foo\",\"vers\":\"1.2.3\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
        ");
    }

    #[test]
    fn test_write_crates() {
        let versions = vec!["0.1.0", "1.0.0-beta.1", "1.0.0", "1.2.3"];
        let crates = versions
            .into_iter()
            .map(|vers| Crate {
                name: "foo".to_string(),
                vers: vers.to_string(),
                deps: vec![],
                cksum: "0123456789asbcdef".to_string(),
                features: Default::default(),
                features2: None,
                yanked: None,
                links: None,
                rust_version: None,
                v: None,
            })
            .collect::<Vec<_>>();

        let mut buffer = Vec::new();
        assert_ok!(write_crates(&crates, &mut buffer));
        assert_ok_eq!(String::from_utf8(buffer), "\
            {\"name\":\"foo\",\"vers\":\"0.1.0\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
            {\"name\":\"foo\",\"vers\":\"1.0.0-beta.1\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
            {\"name\":\"foo\",\"vers\":\"1.0.0\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
            {\"name\":\"foo\",\"vers\":\"1.2.3\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
        ");
    }
}
