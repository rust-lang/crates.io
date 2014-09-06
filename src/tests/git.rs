use std::io::fs;
use std::os;
use std::task;

use git2;
use url::Url;

fn root() -> Path {
    os::getcwd().join("tmp").join(task::name().unwrap())
}

pub fn checkout() -> Path { root().join("checkout") }
pub fn bare() -> Path { root().join("bare") }

pub fn init() {
    let _ = fs::rmdir_recursive(&checkout());
    let _ = fs::rmdir_recursive(&bare());
    // Prepare a bare remote repo
    git2::Repository::init_bare(&bare()).unwrap();

    // Initialize a fresh checkout
    let checkout = git2::Repository::init(&checkout()).unwrap();
    let url = Url::from_file_path(&bare()).unwrap().to_string();

    // Setup the `origin` remote
    let mut origin = checkout.remote_create("origin",
                                            url.as_slice()).unwrap();
    origin.set_pushurl(Some(url.as_slice())).unwrap();
    origin.add_push("refs/heads/master").unwrap();
    origin.save().unwrap();

    // Create an empty initial commit
    let mut config = checkout.config().unwrap();
    config.set_str("user.name", "name").unwrap();
    config.set_str("user.email", "email").unwrap();
    let mut index = checkout.index().unwrap();
    let id = index.write_tree().unwrap();
    let tree = git2::Tree::lookup(&checkout, id).unwrap();
    let sig = git2::Signature::default(&checkout).unwrap();
    git2::Commit::new(&checkout, Some("HEAD"), &sig, &sig,
                      "Initial Commit",
                      &tree, []).unwrap();

    // Push the commit to the remote repo
    let mut origin = checkout.remote_load("origin").unwrap();
    let mut push = origin.push().unwrap();
    push.add_refspec("refs/heads/master").unwrap();
    push.finish().unwrap();
    assert!(push.unpack_ok());
    assert!(!push.statuses().unwrap().iter().any(|s| s.message.is_some()));
    push.update_tips(None, None).unwrap();

    // Set up master to track origin/master
    let branch = git2::Reference::lookup(&checkout, "refs/heads/master");
    let mut branch = git2::Branch::wrap(branch.unwrap());
    branch.set_upstream(Some("origin/master")).unwrap();

}
