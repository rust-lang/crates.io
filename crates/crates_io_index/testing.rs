use anyhow::anyhow;
use git2::build::TreeUpdateBuilder;
use git2::{ErrorCode, FileMode, Repository, Sort};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use tempfile::{Builder, TempDir};
use url::Url;

pub struct UpstreamIndex {
    temp_dir: TempDir,
    pub repository: Mutex<Repository>,
}

impl UpstreamIndex {
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = Builder::new()
            .prefix(thread::current().name().unwrap())
            .tempdir()?;

        debug!(path = %temp_dir.path().display(), "Creating upstream git repository…");
        let bare = git2::Repository::init_opts(
            temp_dir.path(),
            git2::RepositoryInitOptions::new()
                .bare(true)
                .initial_head("master"),
        )?;

        {
            let mut config = bare.config()?;
            config.set_str("user.name", "name")?;
            config.set_str("user.email", "email")?;
        }

        {
            let mut index = bare.index()?;
            let id = index.write_tree()?;
            let tree = bare.find_tree(id)?;
            let sig = bare.signature()?;
            bare.commit(Some("HEAD"), &sig, &sig, "Initial Commit", &tree, &[])?;
        };

        Ok(Self {
            temp_dir,
            repository: Mutex::new(bare),
        })
    }

    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    pub fn url(&self) -> Url {
        Url::from_file_path(self.path()).unwrap()
    }

    pub fn list_commits(&self) -> anyhow::Result<Vec<String>> {
        let repo = self.repository.lock().unwrap();

        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;
        revwalk.push_head()?;

        revwalk
            .map(|result| {
                let oid = result?;
                let commit = repo.find_commit(oid)?;
                let message_bytes = commit.message_bytes();
                let message = String::from_utf8(message_bytes.to_vec())?;
                Ok(message)
            })
            .collect()
    }

    pub fn crate_exists(&self, crate_name: &str) -> anyhow::Result<bool> {
        let repo = self.repository.lock().unwrap();

        let path = crate::Repository::relative_index_file(crate_name);

        let head = repo.head()?;
        let tree = head.peel_to_tree()?;

        match tree.get_path(&path) {
            Ok(_) => Ok(true),
            Err(error) if error.code() == ErrorCode::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    /// Obtain a list of crates from the index HEAD
    pub fn crates_from_index_head(&self, crate_name: &str) -> anyhow::Result<Vec<crate::Crate>> {
        let repo = self.repository.lock().unwrap();

        let path = crate::Repository::relative_index_file(crate_name);

        let head = repo.head()?;
        let tree = head.peel_to_tree()?;
        let blob = tree.get_path(&path)?.to_object(&repo)?.peel_to_blob()?;

        let content = blob.content();

        // The index format consists of one JSON object per line
        // It is not a JSON array
        let lines = std::str::from_utf8(content)?.lines();
        let versions = lines.map(serde_json::from_str).collect::<Result<_, _>>()?;

        Ok(versions)
    }

    pub fn create_empty_commit(&self) -> anyhow::Result<()> {
        let repo = self.repository.lock().unwrap();

        let head = repo.head()?;
        let target = head
            .target()
            .ok_or_else(|| anyhow!("Missing target for HEAD"))?;

        let sig = repo.signature()?;
        let parent = repo.find_commit(target)?;
        let tree = repo.find_tree(parent.tree_id())?;

        repo.commit(Some("HEAD"), &sig, &sig, "empty commit", &tree, &[&parent])?;

        Ok(())
    }

    pub fn read_file(&self, path: &str) -> anyhow::Result<String> {
        let repo = self.repository.lock().unwrap();

        let head = repo.head()?;
        let tree = head.peel_to_tree()?;

        let path = PathBuf::from(path);
        let blob = tree.get_path(&path)?.to_object(&repo)?.peel_to_blob()?;

        let content = blob.content().to_vec();
        let content = String::from_utf8(content)?;

        Ok(content)
    }

    pub fn write_file(&self, path: &str, content: &str) -> anyhow::Result<()> {
        let repo = self.repository.lock().unwrap();

        let head = repo.head()?;
        let head_oid = head
            .target()
            .ok_or_else(|| anyhow!("Missing target for HEAD"))?;

        let parent = repo.find_commit(head_oid)?;
        let tree = repo.find_tree(parent.tree_id())?;

        let message = format!("Write `{path}`");

        let path = PathBuf::from(path);
        let blob_oid = repo.blob(content.as_bytes())?;
        let tree_oid = TreeUpdateBuilder::new()
            .upsert(path, blob_oid, FileMode::Blob)
            .create_updated(&repo, &tree)?;

        let new_tree = repo.find_tree(tree_oid)?;

        let sig = repo.signature()?;
        repo.commit(Some("HEAD"), &sig, &sig, &message, &new_tree, &[&parent])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<UpstreamIndex>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<UpstreamIndex>();
    }
}
