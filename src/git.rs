use std::ascii::AsciiExt;
use std::collections::hash_map::{HashMap, Occupied, Vacant};
use std::io::fs::PathExtensions;
use std::io::util;
use std::io::{Command, BufferedReader, Process, IoResult, File, fs};
use std::io;
use std::os;

use semver;
use flate2::reader::GzDecoder;
use git2;
use serialize::json;

use conduit::{Request, Response};

use app::{App, RequestApp};
use dependency::Kind;
use util::{CargoResult, internal};

#[deriving(Encodable, Decodable)]
pub struct Crate {
    pub name: String,
    pub vers: String,
    pub deps: Vec<Dependency>,
    pub cksum: String,
    pub features: HashMap<String, Vec<String>>,
    pub yanked: Option<bool>,
}

#[deriving(Encodable, Decodable)]
pub struct Dependency {
    pub name: String,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: Option<Kind>,
}

pub fn serve_index(req: &mut Request) -> CargoResult<Response> {
    let mut cmd = Command::new("git");
    cmd.arg("http-backend");

    // Required environment variables
    cmd.env("REQUEST_METHOD",
            req.method().to_string().as_slice().to_ascii_upper());
    cmd.env("GIT_PROJECT_ROOT", &req.app().git_repo_checkout);
    cmd.env("PATH_INFO", req.path().replace("/git/index", ""));
    cmd.env("REMOTE_USER", "");
    cmd.env("REMOTE_ADDR", req.remote_ip().to_string());
    cmd.env("QUERY_STRING", req.query_string().unwrap_or(""));
    cmd.env("CONTENT_TYPE", header(req, "Content-Type"));
    cmd.stderr(::std::io::process::InheritFd(2));
    let mut p = try!(cmd.spawn());

    // Pass in the body of the request (if any)
    //
    // As part of the CGI interface we're required to take care of gzip'd
    // requests. I'm not totally sure that this sequential copy is the best
    // thing to do or actually correct...
    if header(req, "Content-Encoding") == "gzip" {
        let mut body = GzDecoder::new(req.body());
        try!(util::copy(&mut body, &mut p.stdin.take().unwrap()));
    } else {
        try!(util::copy(&mut req.body(), &mut p.stdin.take().unwrap()));
    }

    // Parse the headers coming out, and the pass through the rest of the
    // process back down the stack.
    //
    // Note that we have to be careful to not drop the process which will wait
    // for the process to exit (and we haven't read stdout)
    let mut rdr = BufferedReader::new(p.stdout.take().unwrap());

    let mut headers = HashMap::new();
    for line in rdr.lines() {
        let line = try!(line);
        if line.as_slice() == "\r\n" { break }

        let mut parts = line.as_slice().splitn(2, ':');
        let key = parts.next().unwrap();
        let value = parts.next().unwrap();
        let value = value.slice(1, value.len() - 2);
        match headers.entry(key.to_string()) {
            Occupied(e) => e.into_mut(),
            Vacant(e) => e.set(Vec::new()),
        }.push(value.to_string());
    }

    let (status_code, status_desc) = {
        let line = headers.remove("Status").unwrap_or(Vec::new());
        let line = line.into_iter().next().unwrap_or(String::new());
        let mut parts = line.as_slice().splitn(1, ' ');
        (from_str(parts.next().unwrap_or("")).unwrap_or(200),
         match parts.next() {
             Some("Not Found") => "Not Found",
             _ => "Ok",
         })
    };

    struct ProcessAndBuffer<R> { _p: Process, buf: BufferedReader<R> }
    impl<R: Reader> Reader for ProcessAndBuffer<R> {
        fn read(&mut self, b: &mut [u8]) -> IoResult<uint> { self.buf.read(b) }
    }
    return Ok(Response {
        status: (status_code, status_desc),
        headers: headers,
        body: box ProcessAndBuffer { _p: p, buf: rdr },
    });

    fn header<'a>(req: &'a Request, name: &str) -> &'a str {
        let h = req.headers().find(name).unwrap_or(Vec::new());
        h.as_slice().get(0).map(|s| *s).unwrap_or("")
    }
}

fn index_file(base: &Path, name: &str) -> Path {
    match name.len() {
        1 => base.join("1").join(name),
        2 => base.join("2").join(name),
        3 => base.join("3").join(name.slice_to(1)).join(name),
        _ => base.join(name.slice(0, 2))
                 .join(name.slice(2, 4))
                 .join(name),
    }
}

pub fn add_crate(app: &App, krate: &Crate) -> CargoResult<()> {
    let repo = app.git_repo.lock();
    let repo = &*repo;
    let repo_path = repo.path().dir_path();
    let dst = index_file(&repo_path, krate.name.as_slice());

    commit_and_push(repo, || {
        // Add the crate to its relevant file
        try!(fs::mkdir_recursive(&dst.dir_path(), io::USER_RWX));
        let prev = if dst.exists() {
            try!(File::open(&dst).read_to_string())
        } else {
            String::new()
        };
        let s = json::encode(krate);
        let new = prev + s.as_slice();
        try!(File::create(&dst).write_line(new.as_slice()));

        Ok((format!("Updating crate `{}#{}`", krate.name, krate.vers),
            dst.clone()))
    })
}

pub fn yank(app: &App, krate: &str, version: &semver::Version,
            yanked: bool) -> CargoResult<()> {
    let repo = app.git_repo.lock();
    let repo = &*repo;
    let repo_path = repo.path().dir_path();
    let dst = index_file(&repo_path, krate);

    commit_and_push(repo, || {
        let prev = try!(File::open(&dst).read_to_string());
        let new = prev.as_slice().lines().map(|line| {
            let mut git_crate = try!(json::decode::<Crate>(line).map_err(|_| {
                internal(format!("couldn't decode: `{}`", line))
            }));
            if git_crate.name.as_slice() != krate ||
               git_crate.vers.to_string() != version.to_string() {
                return Ok(line.to_string())
            }
            git_crate.yanked = Some(yanked);
            Ok(json::encode(&git_crate))
        }).collect::<CargoResult<Vec<String>>>();
        let new = try!(new).as_slice().connect("\n");
        try!(File::create(&dst).write_line(new.as_slice()));

        Ok((format!("{} crate `{}#{}`",
                    if yanked {"Yanking"} else {"Unyanking"},
                    krate, version),
            dst.clone()))
    })
}

fn commit_and_push(repo: &git2::Repository,
                   f: || -> CargoResult<(String, Path)>)
                   -> CargoResult<()> {
    let repo_path = repo.path().dir_path();

    // Attempt to commit in a loop. It's possible that we're going to need to
    // rebase our repository, and after that it's possible that we're going to
    // race to commit the changes. For now we just cap out the maximum number of
    // retries at a fixed number.
    for _ in range(0i, 20) {
        let (msg, dst) = try!(f());

        // git add $file
        let mut index = try!(repo.index());
        try!(index.add_path(&dst.path_relative_from(&repo_path).unwrap()));
        try!(index.write());
        let tree_id = try!(index.write_tree());
        let tree = try!(repo.find_tree(tree_id));

        // git commit -m "..."
        let head = try!(repo.head());
        let parent = try!(repo.find_commit(head.target().unwrap()));
        let sig = try!(repo.signature());
        try!(repo.commit(Some("HEAD"), &sig, &sig, msg.as_slice(),
                         &tree, &[&parent]));

        // git push
        let mut origin = try!(repo.find_remote("origin"));
        let cfg = try!(repo.config());
        let ok = try!(with_authentication(origin.url().unwrap(), &cfg, |f| {
            let mut origin = try!(repo.find_remote("origin"));
            let mut callbacks = git2::RemoteCallbacks::new().credentials(f);
            origin.set_callbacks(&mut callbacks);

            let mut push = try!(origin.push());
            try!(push.add_refspec("refs/heads/master"));

            match push.finish() {
                Ok(()) => {}
                Err(..) => return Ok(false)
            }

            if !push.unpack_ok() {
                return Err(internal("failed to push some remote refspecs"))
            }
            try!(push.update_tips(None, None));

            Ok(true)
        }));
        if ok { return Ok(()) }

        // Ok, we need to update, so fetch and reset --hard
        try!(origin.add_fetch("refs/heads/*:refs/heads/*"));
        try!(origin.fetch(&[], None, None));
        let head = try!(repo.head()).target().unwrap();
        let obj = try!(repo.find_object(head, None));
        try!(repo.reset(&obj, git2::ResetType::Hard, None, None));
    }

    Err(internal("Too many rebase failures"))
}

pub fn with_authentication<T>(url: &str,
                              cfg: &git2::Config,
                              f: |git2::Credentials| -> T)
                              -> T {
    let mut cred_helper = git2::CredentialHelper::new(url);
    cred_helper.config(cfg);
    // TODO: read username/pass from the environment
    f(|_url, _username, _allowed| {
        match (os::getenv("GIT_HTTP_USER"), os::getenv("GIT_HTTP_PWD")) {
            (Some(u), Some(p)) => {
                git2::Cred::userpass_plaintext(u.as_slice(), p.as_slice())
            }
            _ => Err(git2::Error::from_str("no authentication set"))
        }
    })
}
