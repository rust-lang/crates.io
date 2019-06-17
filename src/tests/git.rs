use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Once;
use std::thread;

fn root() -> PathBuf {
    env::current_dir()
        .unwrap()
        .join("tmp")
        .join("tests")
        .join(thread::current().name().unwrap())
}

pub fn checkout() -> PathBuf {
    root().join("checkout")
}
pub fn bare() -> PathBuf {
    root().join("bare")
}

pub fn init() {
    static INIT: Once = Once::new();
    let _ = fs::remove_dir_all(&checkout());
    let _ = fs::remove_dir_all(&bare());

    INIT.call_once(|| {
        fs::create_dir_all(root().parent().unwrap()).unwrap();
    });

    let bare = git2::Repository::init_bare(&bare()).unwrap();
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
