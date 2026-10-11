use crate::repo::Repository;
use anyhow::Context;
use std::process::Command;
use tracing::{info, instrument};

const MASTER_REF: &str = "refs/heads/master";

/// Buffers staged index mutations and produces a single commit when finalized.
///
/// Obtain one via [`Repository::commit_builder`]. Stage changes through
/// [`Self::upsert_entry`] and [`Self::remove_entry`], then call
/// [`Self::commit_and_push`] to write the commit and push it to `master`.
/// Dropping the builder without calling `commit_and_push` discards the staged
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
    tub: git2::build::TreeUpdateBuilder,
}

impl<'a> CommitBuilder<'a> {
    pub(crate) fn new(repo: &'a Repository, msg: impl Into<String>) -> anyhow::Result<Self> {
        Ok(Self {
            repo,
            msg: msg.into(),
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
    /// `master` branch on the `origin` remote.
    ///
    /// The push only succeeds if the remote `master` still points at the
    /// parent commit, and the local `master` only advances after the push
    /// succeeds. A rejected push leaves the local clone unchanged.
    ///
    /// Returns `Ok(())` without creating a commit if the resulting tree is
    /// identical to the parent commit's tree (no effective changes).
    #[instrument(skip_all, fields(message = %self.msg))]
    pub fn commit_and_push(mut self) -> Result<(), CommitAndPushError> {
        let gitrepo = self.repo.git_repo();
        let parent = gitrepo
            .find_commit(self.repo.head_oid()?)
            .context("Failed to find HEAD commit")?;
        let parent_tree = parent.tree().context("Failed to load parent tree")?;

        let tree_oid = self
            .tub
            .create_updated(gitrepo, &parent_tree)
            .context("Failed to build updated tree")?;

        if tree_oid == parent.tree_id() {
            info!("No changes to commit");
            return Ok(());
        }

        let sig = gitrepo.signature().context("Failed to read signature")?;
        let tree = gitrepo.find_tree(tree_oid).context("Failed to find tree")?;
        let commit = gitrepo
            .commit(None, &sig, &sig, &self.msg, &tree, &[&parent])
            .context("Failed to create commit")?;

        let parent = parent.id();
        let lease = format!("--force-with-lease={MASTER_REF}:{parent}");
        let refspec = format!("{commit}:{MASTER_REF}");
        let mut command = Command::new("git");
        command.args(["push", &lease, "origin", &refspec]);
        let push = self.repo.run_command(&mut command);
        push.map_err(CommitAndPushError::Push)?;

        gitrepo
            .reference_matching(MASTER_REF, commit, true, parent, "commit_and_push")
            .context("Failed to update local master after push")?;

        Ok(())
    }
}

/// Error returned by [`CommitBuilder::commit_and_push`].
#[derive(Debug, thiserror::Error)]
pub enum CommitAndPushError {
    /// The `git push` command failed, for example because the remote `master`
    /// no longer points at the parent commit.
    #[error("Failed to push the commit")]
    Push(#[source] anyhow::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use crate::repo::{Repository, RepositoryConfig};
    use crate::testing::UpstreamIndex;
    use crate::{Credentials, commit_builder::CommitBuilder};
    use claims::{assert_err, assert_ok_eq};
    use insta::assert_snapshot;

    fn setup() -> (UpstreamIndex, Repository) {
        let upstream = UpstreamIndex::new().unwrap();
        let repo = open_shallow(&upstream);
        (upstream, repo)
    }

    fn open_shallow(upstream: &UpstreamIndex) -> Repository {
        let config = RepositoryConfig {
            index_location: upstream.url(),
            credentials: Credentials::Missing,
        };
        Repository::open_shallow(&config).unwrap()
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
    fn push_advances_local_and_remote_master() {
        let (upstream, repo) = setup();
        let before = repo.head_oid().unwrap();

        let mut builder = commit_builder(&repo, "Create crate `serde`");
        builder.upsert_entry("serde", b"hello\n").unwrap();
        builder.commit_and_push().unwrap();

        let after = repo.head_oid().unwrap();
        assert_ne!(after, before);
        assert_ok_eq!(upstream.branch_oid("master"), after);
    }

    #[test]
    fn push_is_rejected_when_remote_moved_ahead() {
        let (upstream, repo) = setup();
        let local = repo.head_oid().unwrap();
        upstream.write_file("config.json", "{}\n").unwrap();
        let remote = upstream.branch_oid("master").unwrap();

        let mut builder = commit_builder(&repo, "Create crate `serde`");
        builder.upsert_entry("serde", b"hello\n").unwrap();
        let error = assert_err!(builder.commit_and_push());
        assert_snapshot!(error, @"Failed to push the commit");

        assert_ok_eq!(repo.head_oid(), local);
        assert_ok_eq!(upstream.branch_oid("master"), remote);
    }

    #[test]
    fn push_is_rejected_when_remote_was_rewound() {
        let upstream = UpstreamIndex::new().unwrap();
        let initial = upstream.branch_oid("master").unwrap();
        upstream.write_file("config.json", "{}\n").unwrap();
        let repo = open_shallow(&upstream);
        let local = repo.head_oid().unwrap();
        {
            let bare = upstream.repository.lock().unwrap();
            bare.reference("refs/heads/master", initial, true, "rewind")
                .unwrap();
        }

        let mut builder = commit_builder(&repo, "Create crate `serde`");
        builder.upsert_entry("serde", b"hello\n").unwrap();
        let error = assert_err!(builder.commit_and_push());
        assert_snapshot!(error, @"Failed to push the commit");

        assert_ok_eq!(repo.head_oid(), local);
        assert_ok_eq!(upstream.branch_oid("master"), initial);
    }

    #[test]
    fn remove_commits_deletion() {
        let (upstream, repo) = setup();
        upstream.write_file("se/rd/serde", "hello\n").unwrap();
        repo.fetch_and_reset().unwrap();

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
        repo.fetch_and_reset().unwrap();
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
        repo.fetch_and_reset().unwrap();
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
        repo.fetch_and_reset().unwrap();

        let mut builder = commit_builder(&repo, "Create crate `serde`");
        builder.upsert_entry("serde", b"serde\n").unwrap();
        builder.commit_and_push().unwrap();

        assert_ok_eq!(upstream.read_file("config.json"), "{}".to_string());
    }
}
