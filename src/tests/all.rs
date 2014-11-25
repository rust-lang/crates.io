#![feature(macro_rules)]

extern crate "cargo-registry" as cargo_registry;
extern crate "conduit-middleware" as conduit_middleware;
extern crate "conduit-test" as conduit_test;
extern crate conduit;
extern crate curl;
extern crate git2;
extern crate serialize;
extern crate time;
extern crate url;
extern crate semver;

use std::collections::HashMap;
use std::fmt;
use std::io::Command;
use std::io::process::InheritFd;
use std::os;
use std::sync::{Once, ONCE_INIT, Arc};
use serialize::json;

use conduit::Request;
use conduit_test::MockRequest;
use cargo_registry::app::App;
use cargo_registry::db::{mod, RequestTransaction};
use cargo_registry::{User, Crate, Version, Keyword};
use cargo_registry::util::CargoResult;

macro_rules! t( ($e:expr) => (
    match $e {
        Ok(e) => e,
        Err(m) => panic!("{} failed with: {}", stringify!($e), m),
    }
) )

macro_rules! t_resp( ($e:expr) => ({
    t!($e.map_err(|e| (&*e).to_string()))
}) )

macro_rules! ok_resp( ($e:expr) => ({
    let resp = t_resp!($e);
    if !::ok_resp(&resp) { panic!("bad response: {}", resp.status); }
    resp
}) )

macro_rules! bad_resp( ($e:expr) => ({
    let mut resp = t_resp!($e);
    match ::bad_resp(&mut resp) {
        None => panic!("ok response: {}", resp.status),
        Some(b) => b,
    }
}) )

#[deriving(Decodable, Show)]
struct Error { detail: String }
#[deriving(Decodable)]
struct Bad { errors: Vec<Error> }

mod middleware;
mod keyword;
mod krate;
mod user;
mod record;
mod git;
mod version;

fn app() -> (record::Bomb, Arc<App>, conduit_middleware::MiddlewareBuilder) {
    struct NoCommit;
    static mut INIT: Once = ONCE_INIT;
    git::init();

    let (proxy, bomb) = record::proxy();
    let config = cargo_registry::Config {
        s3_bucket: os::getenv("S3_BUCKET").unwrap_or(String::new()),
        s3_access_key: os::getenv("S3_ACCESS_KEY").unwrap_or(String::new()),
        s3_secret_key: os::getenv("S3_SECRET_KEY").unwrap_or(String::new()),
        s3_region: os::getenv("S3_REGION"),
        s3_proxy: Some(proxy),
        session_key: "test".to_string(),
        git_repo_checkout: git::checkout(),
        gh_client_id: "".to_string(),
        gh_client_secret: "".to_string(),
        db_url: env("TEST_DATABASE_URL"),
        env: cargo_registry::Test,
        max_upload_size: 1000,
    };
    unsafe { INIT.doit(|| db_setup(config.db_url.as_slice())); }
    let app = App::new(&config);
    let app = Arc::new(app);
    let mut middleware = cargo_registry::middleware(app.clone());
    middleware.add(NoCommit);
    return (bomb, app, middleware);

    fn env(s: &str) -> String {
        match os::getenv(s) {
            Some(s) => s,
            None => panic!("must have `{}` defined", s),
        }
    }

    fn db_setup(db: &str) {
        let migrate = os::self_exe_name().unwrap().join("../migrate");
        assert!(Command::new(migrate).env("DATABASE_URL", db)
                        .stdout(InheritFd(1))
                        .stderr(InheritFd(2))
                        .status().unwrap().success());
    }

    impl conduit_middleware::Middleware for NoCommit {
        fn after(&self, req: &mut Request,
                 res: Result<conduit::Response, Box<fmt::Show + 'static>>)
                 -> Result<conduit::Response, Box<fmt::Show + 'static>> {
            req.extensions().find::<db::Transaction>()
               .expect("Transaction not present in request")
               .rollback();
            return res;
        }
    }
}

fn req(app: Arc<App>, method: conduit::Method, path: &str) -> MockRequest {
    let mut req = MockRequest::new(method, path);
    req.mut_extensions().insert(db::Transaction::new(app));
    return req;
}

fn ok_resp(r: &conduit::Response) -> bool {
    r.status.val0() == 200
}

fn bad_resp(r: &mut conduit::Response) -> Option<Bad> {
    let bad = json::<Bad>(r);
    if bad.errors.len() == 0 { return None }
    Some(bad)
}

fn json<T>(r: &mut conduit::Response) -> T
           where T: serialize::Decodable<json::Decoder, json::DecoderError> {
    let data = r.body.read_to_end().unwrap();
    let s = std::str::from_utf8(data.as_slice()).unwrap();
    let j = match json::from_str(s) {
        Ok(t) => t,
        Err(e) => panic!("failed to decode: {}\n{}", e, s),
    };
    let j = fixup(j);
    let s = j.to_string();
    return match json::decode(s.as_slice()) {
        Ok(t) => t,
        Err(e) => panic!("failed to decode: {}\n{}", e, s),
    };


    fn fixup(json: json::Json) -> json::Json {
        match json {
            json::Object(object) => {
                json::Object(object.into_iter().map(|(k, v)| {
                    let k = if k.as_slice() == "crate" {
                        "krate".to_string()
                    } else {
                        k
                    };
                    (k, fixup(v))
                }).collect())
            }
            json::Array(list) => {
                json::Array(list.into_iter().map(fixup).collect())
            }
            j => j,
        }
    }
}

fn user(login: &str) -> User {
    User {
        id: 10000,
        gh_login: login.to_string(),
        email: None,
        name: None,
        avatar: None,
        gh_access_token: User::new_api_token(), // just randomize it
        api_token: User::new_api_token(),
    }
}

fn krate(name: &str) -> Crate {
    cargo_registry::krate::Crate {
        id: 10000,
        name: name.to_string(),
        user_id: 100,
        updated_at: time::now().to_timespec(),
        created_at: time::now().to_timespec(),
        downloads: 10,
        max_version: semver::Version::parse("0.0.0").unwrap(),
        documentation: None,
        homepage: None,
        description: None,
        readme: None,
        keywords: Vec::new(),
        license: None,
        repository: None,
    }
}

fn mock_user(req: &mut Request, u: User) -> User {
    let u = User::find_or_insert(req.tx().unwrap(),
                                 u.gh_login.as_slice(),
                                 u.email.as_ref().map(|s| s.as_slice()),
                                 u.name.as_ref().map(|s| s.as_slice()),
                                 u.avatar.as_ref().map(|s| s.as_slice()),
                                 u.gh_access_token.as_slice(),
                                 u.api_token.as_slice()).unwrap();
    req.mut_extensions().insert(u.clone());
    return u;
}

fn mock_crate(req: &mut Request, krate: Crate) -> Crate {
    let (c, v) = mock_crate_vers(req, krate, &semver::Version::parse("1.0.0").unwrap());
    v.unwrap();
    c
}
fn mock_crate_vers(req: &mut Request, krate: Crate, v: &semver::Version)
                   -> (Crate, CargoResult<Version>) {
    let user = req.extensions().find::<User>().unwrap();
    let mut krate = Crate::find_or_insert(req.tx().unwrap(), krate.name.as_slice(),
                                      user.id, &krate.description,
                                      &krate.homepage,
                                      &krate.documentation,
                                      &krate.readme,
                                      krate.keywords.as_slice(),
                                      &krate.repository,
                                      &krate.license,
                                      &None).unwrap();
    Keyword::update_crate(req.tx().unwrap(), &krate,
                          krate.keywords.as_slice()).unwrap();
    let v = krate.add_version(req.tx().unwrap(), v, &HashMap::new(), &[]);
    (krate, v)
}

fn mock_keyword(req: &mut Request, name: &str) -> Keyword {
    Keyword::find_or_insert(req.tx().unwrap(), name).unwrap()
}

fn logout(req: &mut Request) {
    req.mut_extensions().pop::<User>();
}
