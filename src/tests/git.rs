use std::fs;
use std::env;
use std::thread;
use std::path::PathBuf;
use std::sync::{Once, ONCE_INIT};

use git2;
use url::Url;

fn root() -> PathBuf {
    env::current_dir()
        .unwrap()
        .join("tmp")
        .join(thread::current().name().unwrap())
}

pub fn checkout() -> PathBuf {
    root().join("checkout")
}
pub fn bare() -> PathBuf {
    root().join("bare")
}

pub fn init() {
    static INIT: Once = ONCE_INIT;
    let _ = fs::remove_dir_all(&checkout());
    let _ = fs::remove_dir_all(&bare());

    INIT.call_once(|| {
        fs::create_dir_all(root().parent().unwrap()).unwrap();
    });

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
