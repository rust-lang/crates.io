use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use semver;
use git2;
use rustc_serialize::json;

use app::App;
use dependency::Kind;
use util::{CargoResult, internal};

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct Crate {
    pub name: String,
    pub vers: String,
    pub deps: Vec<Dependency>,
    pub cksum: String,
    pub features: HashMap<String, Vec<String>>,
    pub yanked: Option<bool>,
}

#[derive(RustcEncodable, RustcDecodable, Debug)]
pub struct Dependency {
    pub name: String,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: Option<Kind>,
}

fn index_file(base: &Path, name: &str) -> PathBuf {
    let name = name.chars()
        .flat_map(|c| c.to_lowercase())
        .collect::<String>();
    match name.len() {
        1 => base.join("1").join(&name),
        2 => base.join("2").join(&name),
        3 => base.join("3").join(&name[..1]).join(&name),
        _ => base.join(&name[0..2]).join(&name[2..4]).join(&name),
    }
}

pub fn add_crate(app: &App, krate: &Crate) -> CargoResult<()> {
    let repo = app.git_repo.lock().unwrap();
    let repo = &*repo;
    let repo_path = repo.workdir().unwrap();
    let dst = index_file(repo_path, &krate.name);

    commit_and_push(repo, || {
        // Add the crate to its relevant file
        fs::create_dir_all(dst.parent().unwrap())?;
        let mut prev = String::new();
        if fs::metadata(&dst).is_ok() {
            File::open(&dst).and_then(
                |mut f| f.read_to_string(&mut prev),
            )?;
        }
        let s = json::encode(krate).unwrap();
        let new = prev + &s;
        let mut f = File::create(&dst)?;
        f.write_all(new.as_bytes())?;
        f.write_all(b"\n")?;

        Ok((
            format!("Updating crate `{}#{}`", krate.name, krate.vers),
            dst.clone(),
        ))
    })
}

pub fn yank(app: &App, krate: &str, version: &semver::Version, yanked: bool) -> CargoResult<()> {
    let repo = app.git_repo.lock().unwrap();
    let repo_path = repo.workdir().unwrap();
    let dst = index_file(repo_path, krate);

    commit_and_push(&repo, || {
        let mut prev = String::new();
        File::open(&dst).and_then(
            |mut f| f.read_to_string(&mut prev),
        )?;
        let new = prev.lines()
            .map(|line| {
                let mut git_crate = json::decode::<Crate>(line).map_err(|_| {
                    internal(&format_args!("couldn't decode: `{}`", line))
                })?;
                if git_crate.name != krate || git_crate.vers != version.to_string() {
                    return Ok(line.to_string());
                }
                git_crate.yanked = Some(yanked);
                Ok(json::encode(&git_crate).unwrap())
            })
            .collect::<CargoResult<Vec<String>>>();
        let new = new?.join("\n");
        let mut f = File::create(&dst)?;
        f.write_all(new.as_bytes())?;
        f.write_all(b"\n")?;

        Ok((
            format!(
                "{} crate `{}#{}`",
                if yanked { "Yanking" } else { "Unyanking" },
                krate,
                version
            ),
            dst.clone(),
        ))
    })
}

fn commit_and_push<F>(repo: &git2::Repository, mut f: F) -> CargoResult<()>
where
    F: FnMut() -> CargoResult<(String, PathBuf)>,
{
    let repo_path = repo.workdir().unwrap();

    // Attempt to commit in a loop. It's possible that we're going to need to
    // rebase our repository, and after that it's possible that we're going to
    // race to commit the changes. For now we just cap out the maximum number of
    // retries at a fixed number.
    for _ in 0..20 {
        let (msg, dst) = f()?;

        // git add $file
        let mut index = repo.index()?;
        let mut repo_path = repo_path.iter();
        let dst = dst.iter()
            .skip_while(|s| Some(*s) == repo_path.next())
            .collect::<PathBuf>();
        index.add_path(&dst)?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        // git commit -m "..."
        let head = repo.head()?;
        let parent = repo.find_commit(head.target().unwrap())?;
        let sig = repo.signature()?;
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &msg,
            &tree,
            &[&parent],
        )?;

        // git push
        let mut ref_status = None;
        let mut origin = repo.find_remote("origin")?;
        let res = {
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.credentials(credentials);
            callbacks.push_update_reference(|refname, status| {
                assert_eq!(refname, "refs/heads/master");
                ref_status = status.map(|s| s.to_string());
                Ok(())
            });
            let mut opts = git2::PushOptions::new();
            opts.remote_callbacks(callbacks);
            origin.push(&["refs/heads/master"], Some(&mut opts))
        };
        match res {
            Ok(()) if ref_status.is_none() => return Ok(()),
            Ok(()) => info!("failed to push a ref: {:?}", ref_status),
            Err(e) => info!("failure to push: {}", e),
        }

        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(credentials);
        origin.update_tips(
            Some(&mut callbacks),
            true,
            git2::AutotagOption::Unspecified,
            None,
        )?;

        // Ok, we need to update, so fetch and reset --hard
        origin.fetch(&["refs/heads/*:refs/heads/*"], None, None)?;
        let head = repo.head()?.target().unwrap();
        let obj = repo.find_object(head, None)?;
        repo.reset(&obj, git2::ResetType::Hard, None)?;
    }

    Err(internal("Too many rebase failures"))
}

pub fn credentials(
    _user: &str,
    _user_from_url: Option<&str>,
    _cred: git2::CredentialType,
) -> Result<git2::Cred, git2::Error> {
    match (env::var("GIT_HTTP_USER"), env::var("GIT_HTTP_PWD")) {
        (Ok(u), Ok(p)) => git2::Cred::userpass_plaintext(&u, &p),
        _ => Err(git2::Error::from_str("no authentication set")),
    }
}
