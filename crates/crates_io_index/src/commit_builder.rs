use crate::repo::Repository;
use anyhow::Context;
use std::process::Command;
use tracing::{info, instrument};

/// Buffers staged index mutations and produces a single commit when finalized.
///
/// Obtain one via [`Repository::commit_builder`] or
/// [`Repository::commit_builder_to`]. Stage changes through [`Self::upsert_entry`]
/// and [`Self::remove_entry`], then call [`Self::commit_and_push`] or
/// [`Self::commit_and_push_with_lease`] to write the commit and push it to the
/// target branch. Dropping the builder without either call discards the staged
/// operations; any blobs that were written to the ODB become unreachable and
/// will be cleaned up by `git gc`.
///
/// ### Limitation
///
/// Each path may be touched at most once per builder: mixing or repeating
/// `upsert_entry` / `remove_entry` for the same `name` is unsupported and
/// will fail at commit time. libgit2's `git_tree_create_updated` rejects
/// duplicate operations on the same path, upserts of previously-removed
/// paths, and type-changing upserts.
pub struct CommitBuilder<'a> {
    repo: &'a Repository,
    msg: String,
    branch: String,
    tub: git2::build::TreeUpdateBuilder,
}

impl<'a> CommitBuilder<'a> {
    pub(crate) fn new(
        repo: &'a Repository,
        msg: impl Into<String>,
        branch: impl Into<String>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            repo,
            msg: msg.into(),
            branch: branch.into(),
            tub: git2::build::TreeUpdateBuilder::new(),
        })
    }

    /// Stages `bytes` as the contents of the index entry for `name`, creating
    /// or overwriting the entry.
    pub fn upsert_entry(&mut self, name: &str, bytes: &[u8]) -> anyhow::Result<()> {
        let oid = self
            .repo
            .git_repo()
            .blob(bytes)
            .with_context(|| format!("Failed to write blob for `{name}`"))?;
        let path = Repository::relative_index_file_for_url(name);
        self.tub.upsert(&path, oid, git2::FileMode::Blob);
        Ok(())
    }

    /// Stages removal of the index entry for `name`.
    pub fn remove_entry(&mut self, name: &str) -> anyhow::Result<()> {
        let path = Repository::relative_index_file_for_url(name);
        self.tub.remove(&path);
        Ok(())
    }

    /// Writes the staged changes as a new commit and pushes it to the
    /// configured branch on the `origin` remote.
    ///
    /// Returns `Ok(())` without creating a commit if the resulting tree is
    /// identical to the parent commit's tree (no effective changes).
    #[instrument(skip_all, fields(message = %self.msg, branch = %self.branch))]
    pub fn commit_and_push(self) -> anyhow::Result<()> {
        self.finish(false)
    }

    /// Pushes a commit only if the remote branch still points at the local
    /// parent commit. The local HEAD advances only after the push succeeds.
    /// If updating local HEAD fails after the push, the remote may have
    /// advanced despite an error; callers should refresh before retrying.
    ///
    /// Returns `Ok(())` without creating a commit if the resulting tree is
    /// identical to the parent commit's tree (no effective changes).
    #[instrument(skip_all, fields(message = %self.msg, branch = %self.branch))]
    pub fn commit_and_push_with_lease(self) -> anyhow::Result<()> {
        self.finish(true)
    }

    /// Builds one commit and advances local HEAD before or after the push.
    fn finish(mut self, with_lease: bool) -> anyhow::Result<()> {
        let gitrepo = self.repo.git_repo();
        let parent = gitrepo.find_commit(self.repo.head_oid()?)?;
        let parent_tree = parent.tree().context("Failed to load parent tree")?;

        let tree_oid = self
            .tub
            .create_updated(gitrepo, &parent_tree)
            .context("Failed to build updated tree")?;

        if tree_oid == parent.tree_id() {
            info!("No changes to commit");
            return Ok(());
        }

        let sig = gitrepo.signature()?;
        let tree = gitrepo.find_tree(tree_oid)?;
        let update_ref = if with_lease { None } else { Some("HEAD") };
        let commit_oid = gitrepo.commit(update_ref, &sig, &sig, &self.msg, &tree, &[&parent])?;

        let branch = format!("refs/heads/{}", self.branch);
        let refspec = format!("{commit_oid}:{branch}");
        let mut command = Command::new("git");
        command.arg("push");
        if with_lease {
            let lease = format!("--force-with-lease={branch}:{}", parent.id());
            command.arg(lease);
        }
        command.args(["origin", &refspec]);
        self.repo.run_command(&mut command)?;

        if with_lease {
            let head_ref = "refs/heads/master";
            gitrepo
                .reference_matching(head_ref, commit_oid, true, parent.id(), "push with lease")
                .context("Failed to update local index head after push")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::repo::{Repository, RepositoryConfig};
    use crate::testing::UpstreamIndex;
    use crate::{Credentials, commit_builder::CommitBuilder};
    use claims::{assert_err, assert_ok_eq};

    fn setup() -> (UpstreamIndex, Repository) {
        let upstream = UpstreamIndex::new().unwrap();
        let config = RepositoryConfig {
            index_location: upstream.url(),
            credentials: Credentials::Missing,
        };
        let repo = Repository::open(&config).unwrap();
        (upstream, repo)
    }

    fn setup_shallow() -> (UpstreamIndex, Repository) {
        let upstream = UpstreamIndex::new().unwrap();
        let config = RepositoryConfig {
            index_location: upstream.url(),
            credentials: Credentials::Missing,
        };
        let repo = Repository::open_shallow(&config).unwrap();
        (upstream, repo)
    }

    fn commit_builder<'a>(repo: &'a Repository, msg: &str) -> CommitBuilder<'a> {
        repo.commit_builder(msg).unwrap()
    }

    #[test]
    fn empty_builder_does_not_commit() {
        let (upstream, repo) = setup();
        let before = upstream.list_commits().unwrap();

        commit_builder(&repo, "should not appear")
            .commit_and_push()
            .unwrap();

        assert_eq!(upstream.list_commits().unwrap(), before);
    }

    #[test]
    fn upsert_creates_commit() {
        let (upstream, repo) = setup();

        let mut builder = commit_builder(&repo, "Create crate `serde`");
        builder.upsert_entry("serde", b"hello\n").unwrap();
        builder.commit_and_push().unwrap();

        assert_ok_eq!(
            upstream.list_commits(),
            vec!["Initial Commit", "Create crate `serde`"]
        );
        assert_ok_eq!(upstream.read_file("se/rd/serde"), "hello\n".to_string());
    }

    #[test]
    fn remove_commits_deletion() {
        let (upstream, repo) = setup();
        upstream.write_file("se/rd/serde", "hello\n").unwrap();
        repo.reset_head().unwrap();

        let mut builder = commit_builder(&repo, "Delete crate `serde`");
        builder.remove_entry("serde").unwrap();
        builder.commit_and_push().unwrap();

        assert_ok_eq!(upstream.crate_exists("serde"), false);
        assert_eq!(
            upstream.list_commits().unwrap().last().unwrap(),
            "Delete crate `serde`"
        );
    }

    #[test]
    fn upsert_with_identical_content_does_not_commit() {
        let (upstream, repo) = setup();
        upstream.write_file("se/rd/serde", "hello\n").unwrap();
        repo.reset_head().unwrap();
        let before = upstream.list_commits().unwrap();

        let mut builder = commit_builder(&repo, "no-op upsert");
        builder.upsert_entry("serde", b"hello\n").unwrap();
        builder.commit_and_push().unwrap();

        assert_eq!(upstream.list_commits().unwrap(), before);
    }

    #[test]
    fn multi_entry_changes_produce_single_commit() {
        let (upstream, repo) = setup();
        upstream.write_file("ol/d_/old_crate", "old\n").unwrap();
        repo.reset_head().unwrap();
        let before_count = upstream.list_commits().unwrap().len();

        let mut builder = commit_builder(&repo, "Bulk update");
        builder.upsert_entry("serde", b"serde\n").unwrap();
        builder.upsert_entry("anyhow", b"anyhow\n").unwrap();
        builder.remove_entry("old_crate").unwrap();
        builder.commit_and_push().unwrap();

        let commits = upstream.list_commits().unwrap();
        assert_eq!(commits.len(), before_count + 1);
        assert_eq!(commits.last().unwrap(), "Bulk update");
        assert_ok_eq!(upstream.read_file("se/rd/serde"), "serde\n".to_string());
        assert_ok_eq!(upstream.read_file("an/yh/anyhow"), "anyhow\n".to_string());
        assert_ok_eq!(upstream.crate_exists("old_crate"), false);
    }

    #[test]
    fn top_level_files_are_preserved_across_commits() {
        let (upstream, repo) = setup();
        upstream.write_file("config.json", "{}").unwrap();
        repo.reset_head().unwrap();

        let mut builder = commit_builder(&repo, "Create crate `serde`");
        builder.upsert_entry("serde", b"serde\n").unwrap();
        builder.commit_and_push().unwrap();

        assert_ok_eq!(upstream.read_file("config.json"), "{}".to_string());
    }

    #[test]
    fn commit_builder_to_targets_specific_branch() {
        let (upstream, repo) = setup();

        // Create the target branch on the upstream so the push has somewhere
        // to land. We simply start it at the current HEAD.
        {
            let bare = upstream.repository.lock().unwrap();
            let head = bare.head().unwrap().target().unwrap();
            bare.reference("refs/heads/side", head, false, "set up side branch")
                .unwrap();
        }

        let mut builder = repo.commit_builder_to("Sideways", "side").unwrap();
        builder.upsert_entry("serde", b"serde\n").unwrap();
        builder.commit_and_push().unwrap();

        // master is untouched
        assert_ok_eq!(upstream.list_commits(), vec!["Initial Commit".to_string()]);

        // the side branch has the new commit
        let bare = upstream.repository.lock().unwrap();
        let side = bare.find_reference("refs/heads/side").unwrap();
        let commit = bare.find_commit(side.target().unwrap()).unwrap();
        assert_eq!(commit.message().unwrap(), "Sideways");
    }

    #[test]
    fn leased_push_advances_local_and_remote_heads() {
        let (upstream, repo) = setup_shallow();
        let before = repo.head_oid().unwrap();

        let mut builder = commit_builder(&repo, "Create crate `serde`");
        builder.upsert_entry("serde", b"hello\n").unwrap();
        builder.commit_and_push_with_lease().unwrap();

        let after = repo.head_oid().unwrap();
        assert_ne!(after, before);
        assert_eq!(upstream.branch_oid("master").unwrap(), after);
        assert_ok_eq!(upstream.read_file("se/rd/serde"), "hello\n".to_string());
    }

    #[test]
    fn leased_push_rejects_advanced_remote_without_advancing_local_head() {
        let (upstream, repo) = setup_shallow();
        let before = repo.head_oid().unwrap();
        upstream.write_file("config.json", "{}\n").unwrap();
        let remote = upstream.branch_oid("master").unwrap();

        let mut builder = commit_builder(&repo, "Create crate `serde`");
        builder.upsert_entry("serde", b"hello\n").unwrap();
        assert_err!(builder.commit_and_push_with_lease());

        assert_eq!(repo.head_oid().unwrap(), before);
        assert_eq!(upstream.branch_oid("master").unwrap(), remote);
        assert_ok_eq!(upstream.crate_exists("serde"), false);
    }

    #[test]
    fn leased_push_rejects_remote_rewind_without_advancing_local_head() {
        let upstream = UpstreamIndex::new().unwrap();
        let initial = upstream.branch_oid("master").unwrap();
        upstream.write_file("config.json", "{}\n").unwrap();
        let config = RepositoryConfig {
            index_location: upstream.url(),
            credentials: Credentials::Missing,
        };
        let repo = Repository::open_shallow(&config).unwrap();
        let before = repo.head_oid().unwrap();
        upstream
            .repository
            .lock()
            .unwrap()
            .reference("refs/heads/master", initial, true, "rewind")
            .unwrap();

        let mut builder = commit_builder(&repo, "Create crate `serde`");
        builder.upsert_entry("serde", b"hello\n").unwrap();
        assert_err!(builder.commit_and_push_with_lease());

        assert_eq!(repo.head_oid().unwrap(), before);
        assert_eq!(upstream.branch_oid("master").unwrap(), initial);
        assert_ok_eq!(upstream.crate_exists("serde"), false);
    }
}
