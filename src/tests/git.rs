use std::fs;
use std::env;
use std::thread;
use std::path::PathBuf;
use std::sync::{Once, ONCE_INIT};

use git2;
use url::Url;

fn root() -> PathBuf {
    env::current_dir().unwrap().join("tmp").join(thread::current().name().unwrap())
}

pub fn checkout() -> PathBuf { root().join("checkout") }
pub fn bare() -> PathBuf { root().join("bare") }

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
    let mut origin = checkout.remote("origin", url.as_slice()).unwrap();
    origin.set_pushurl(Some(url.as_slice())).unwrap();
    origin.add_push("refs/heads/master").unwrap();
    origin.save().unwrap();

    // Create an empty initial commit
    let mut config = checkout.config().unwrap();
    config.set_str("user.name", "name").unwrap();
    config.set_str("user.email", "email").unwrap();
    let mut index = checkout.index().unwrap();
    let id = index.write_tree().unwrap();
    let tree = checkout.find_tree(id).unwrap();
    let sig = checkout.signature().unwrap();
    checkout.commit(Some("HEAD"), &sig, &sig,
                    "Initial Commit",
                    &tree, &[]).unwrap();

    // Push the commit to the remote repo
    let mut origin = checkout.find_remote("origin").unwrap();
    let mut push = origin.push().unwrap();
    push.add_refspec("refs/heads/master").unwrap();
    push.finish().unwrap();
    assert!(!push.statuses().unwrap().iter().any(|s| s.message.is_some()));
    push.update_tips(None, None).unwrap();

    // Set up master to track origin/master
    let branch = checkout.find_reference("refs/heads/master");
    let mut branch = git2::Branch::wrap(branch.unwrap());
    branch.set_upstream(Some("origin/master")).unwrap();

}
