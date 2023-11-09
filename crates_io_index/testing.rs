use anyhow::anyhow;
use git2::{ErrorCode, Repository, Sort};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Once;
use std::thread;
use url::Url;

pub struct UpstreamIndex {
    pub repository: Repository,
}

impl UpstreamIndex {
    pub fn new() -> anyhow::Result<Self> {
        init();

        let thread_local_path = bare();
        let repository = Repository::open_bare(thread_local_path)?;
        Ok(Self { repository })
    }

    pub fn url(&self) -> Url {
        Url::from_file_path(self.repository.path()).unwrap()
    }

    pub fn list_commits(&self) -> anyhow::Result<Vec<String>> {
        let mut revwalk = self.repository.revwalk()?;
        revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)?;
        revwalk.push_head()?;

        revwalk
            .map(|result| {
                let oid = result?;
                let commit = self.repository.find_commit(oid)?;
                let message_bytes = commit.message_bytes();
                let message = String::from_utf8(message_bytes.to_vec())?;
                Ok(message)
            })
            .collect()
    }

    pub fn crate_exists(&self, crate_name: &str) -> anyhow::Result<bool> {
        let repo = &self.repository;

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
        let repo = &self.repository;

        let path = crate::Repository::relative_index_file(crate_name);

        let head = repo.head()?;
        let tree = head.peel_to_tree()?;
        let blob = tree.get_path(&path)?.to_object(repo)?.peel_to_blob()?;

        let content = blob.content();

        // The index format consists of one JSON object per line
        // It is not a JSON array
        let lines = std::str::from_utf8(content)?.lines();
        let versions = lines.map(serde_json::from_str).collect::<Result<_, _>>()?;

        Ok(versions)
    }

    pub fn create_empty_commit(&self) -> anyhow::Result<()> {
        let repo = &self.repository;

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
}

fn root() -> PathBuf {
    env::current_dir()
        .unwrap()
        .join("tmp")
        .join("tests")
        .join(thread::current().name().unwrap())
}

fn bare() -> PathBuf {
    root().join("bare")
}

fn init() {
    static INIT: Once = Once::new();
    let _ = fs::remove_dir_all(bare());

    INIT.call_once(|| {
        fs::create_dir_all(root().parent().unwrap()).unwrap();
    });

    let bare = git2::Repository::init_opts(
        bare(),
        git2::RepositoryInitOptions::new()
            .bare(true)
            .initial_head("master"),
    )
    .unwrap();
    let mut config = bare.config().unwrap();
    config.set_str("user.name", "name").unwrap();
    config.set_str("user.email", "email").unwrap();
    let mut index = bare.index().unwrap();
    let id = index.write_tree().unwrap();
    let tree = bare.find_tree(id).unwrap();
    let sig = bare.signature().unwrap();
    bare.commit(Some("HEAD"), &sig, &sig, "Initial Commit", &tree, &[])
        .unwrap();
}
