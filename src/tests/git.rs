use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;

use git2;
use url::Url;
use uuid::Uuid;

fn root() -> PathBuf {
    env::current_dir()
        .unwrap()
        .join("tmp")
        .join(thread::current().name().unwrap().replace("::", "_"))
}

pub fn checkout() -> PathBuf {
    root().join("checkout")
}
pub fn bare() -> PathBuf {
    root().join("bare")
}

#[cfg(target_os = "windows")]
fn remove_dir_all(path: &Path) -> Result<(), io::Error> {
    fn rename_temp(path: &Path) -> Result<PathBuf, io::Error> {
        let temp_name = Uuid::new_v4().hyphenated().to_string();
        let temp_path = env::temp_dir().join(temp_name);
        fs::rename(path, &temp_path)?;
        Ok(temp_path)
    }

    fn remove_file(path: &Path) -> Result<(), io::Error> {
        let temp_path = rename_temp(path)?;
        fs::remove_file(&temp_path)
    }

    fn remove_dir(path: &Path) -> Result<(), io::Error> {
        let temp_path = rename_temp(path)?;
        fs::remove_dir(&temp_path)
    }

    for cursor in fs::read_dir(path)? {
        let entry = cursor?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            remove_dir_all(&entry.path())?;
        } else {
            let mut permissions = entry.metadata()?.permissions();
            if permissions.readonly() {
                permissions.set_readonly(false);
                fs::set_permissions(entry.path(), permissions)?;
            }
            remove_file(&entry.path())?;
        }
    }
    remove_dir(path)
}

#[cfg(not(target_os = "windows"))]
fn remove_dir_all(path: &Path) -> Result<(), io::Error> {
    fs::remove_dir_all(path)
}

pub fn init() {
    if let Err(e) = remove_dir_all(&checkout()) {
        println!("Errored: {:?}", e);
    }
    if let Err(e) = remove_dir_all(&bare()) {
        println!("Errored: {:?}", e);
    }
    if let Err(e) = fs::create_dir_all(&checkout()) {
        println!("Errored: {:?}", e);
    }
    if let Err(e) = fs::create_dir_all(&bare()) {
        println!("Errored: {:?}", e);
    }

    // Prepare a bare remote repo
    {
        let bare = git2::Repository::init_bare(&bare()).unwrap();
        let mut config = bare.config().unwrap();
        config.set_str("user.name", "name").unwrap();
        config.set_str("user.email", "email").unwrap();
    }

    // Initialize a fresh checkout
    let checkout = git2::Repository::init(&checkout()).unwrap();
    let url = Url::from_file_path(&*bare()).ok().unwrap().to_string();

    // Setup the `origin` remote
    checkout.remote_set_url("origin", &url).unwrap();
    checkout.remote_set_pushurl("origin", Some(&url)).unwrap();
    checkout
        .remote_add_push("origin", "refs/heads/master")
        .unwrap();

    // Create an empty initial commit
    let mut config = checkout.config().unwrap();
    config.set_str("user.name", "name").unwrap();
    config.set_str("user.email", "email").unwrap();
    let mut index = checkout.index().unwrap();
    let id = index.write_tree().unwrap();
    let tree = checkout.find_tree(id).unwrap();
    let sig = checkout.signature().unwrap();
    checkout
        .commit(Some("HEAD"), &sig, &sig, "Initial Commit", &tree, &[])
        .unwrap();

    // Push the commit to the remote repo
    let mut origin = checkout.find_remote("origin").unwrap();
    origin.push(&["refs/heads/master"], None).unwrap();
}
