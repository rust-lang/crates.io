#![deny(warnings)]
#![feature(io, path_ext, convert)]

extern crate cargo_registry;
extern crate conduit_middleware;
extern crate conduit_test;
extern crate rustc_serialize;
extern crate conduit;
extern crate curl;
extern crate git2;
extern crate time;
extern crate url;
extern crate semver;

use std::collections::HashMap;
use std::error::Error as StdError;
use std::process::Command;
use std::env;
use std::sync::{Once, ONCE_INIT, Arc};
use rustc_serialize::json::{self, Json};

use conduit::Request;
use conduit_test::MockRequest;
use cargo_registry::app::App;
use cargo_registry::db::{self, RequestTransaction};
use cargo_registry::dependency::Kind;
use cargo_registry::{User, Crate, Version, Keyword, Dependency};

macro_rules! t{ ($e:expr) => (
    match $e {
        Ok(e) => e,
        Err(m) => panic!("{} failed with: {}", stringify!($e), m),
    }
) }

macro_rules! t_resp{ ($e:expr) => ({
    t!($e)
}) }

macro_rules! ok_resp{ ($e:expr) => ({
    let resp = t_resp!($e);
    if !::ok_resp(&resp) { panic!("bad response: {:?}", resp.status); }
    resp
}) }

macro_rules! bad_resp{ ($e:expr) => ({
    let mut resp = t_resp!($e);
    match ::bad_resp(&mut resp) {
        None => panic!("ok response: {:?}", resp.status),
        Some(b) => b,
    }
}) }

#[derive(RustcDecodable, Debug)]
struct Error { detail: String }
#[derive(RustcDecodable)]
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
    static INIT: Once = ONCE_INIT;
    git::init();

    let (proxy, bomb) = record::proxy();
    let config = cargo_registry::Config {
        s3_bucket: env::var("S3_BUCKET").unwrap_or(String::new()),
        s3_access_key: env::var("S3_ACCESS_KEY").unwrap_or(String::new()),
        s3_secret_key: env::var("S3_SECRET_KEY").unwrap_or(String::new()),
        s3_region: env::var("S3_REGION").ok(),
        s3_proxy: Some(proxy),
        session_key: "test".to_string(),
        git_repo_checkout: git::checkout(),
        gh_client_id: "".to_string(),
        gh_client_secret: "".to_string(),
        db_url: env("TEST_DATABASE_URL"),
        env: cargo_registry::Env::Test,
        max_upload_size: 1000,
    };
    INIT.call_once(|| db_setup(&config.db_url));
    let app = App::new(&config);
    let app = Arc::new(app);
    let mut middleware = cargo_registry::middleware(app.clone());
    middleware.add(NoCommit);
    return (bomb, app, middleware);

    fn env(s: &str) -> String {
        match env::var(s).ok() {
            Some(s) => s,
            None => panic!("must have `{}` defined", s),
        }
    }

    fn db_setup(db: &str) {
        let migrate = t!(env::current_exe()).parent().unwrap().join("migrate");
        assert!(t!(Command::new(&migrate).env("DATABASE_URL", db)
                           .status()).success());
    }

    impl conduit_middleware::Middleware for NoCommit {
        fn after(&self, req: &mut Request,
                 res: Result<conduit::Response, Box<StdError+Send>>)
                 -> Result<conduit::Response, Box<StdError+Send>> {
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
    r.status.0 == 200
}

fn bad_resp(r: &mut conduit::Response) -> Option<Bad> {
    let bad = json::<Bad>(r);
    if bad.errors.len() == 0 { return None }
    Some(bad)
}

fn json<T: rustc_serialize::Decodable>(r: &mut conduit::Response) -> T {
    let mut data = Vec::new();
    r.body.read_to_end(&mut data).unwrap();
    let s = std::str::from_utf8(&data).unwrap();
    let j = match Json::from_str(s) {
        Ok(t) => t,
        Err(e) => panic!("failed to decode: {:?}\n{}", e, s),
    };
    let j = fixup(j);
    let s = j.to_string();
    return match json::decode(&s) {
        Ok(t) => t,
        Err(e) => panic!("failed to decode: {:?}\n{}", e, s),
    };


    fn fixup(json: Json) -> Json {
        match json {
            Json::Object(object) => {
                Json::Object(object.into_iter().map(|(k, v)| {
                    let k = if k == "crate" {
                        "krate".to_string()
                    } else {
                        k
                    };
                    (k, fixup(v))
                }).collect())
            }
            Json::Array(list) => {
                Json::Array(list.into_iter().map(fixup).collect())
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
                                 &u.gh_login,
                                 u.email.as_ref().map(|s| &s[..]),
                                 u.name.as_ref().map(|s| &s[..]),
                                 u.avatar.as_ref().map(|s| &s[..]),
                                 &u.gh_access_token,
                                 &u.api_token).unwrap();
    req.mut_extensions().insert(u.clone());
    return u;
}

fn mock_crate(req: &mut Request, krate: Crate) -> (Crate, Version) {
    mock_crate_vers(req, krate, &semver::Version::parse("1.0.0").unwrap())
}
fn mock_crate_vers(req: &mut Request, krate: Crate, v: &semver::Version)
                   -> (Crate, Version) {
    let user = req.extensions().find::<User>().unwrap();
    let mut krate = Crate::find_or_insert(req.tx().unwrap(), &krate.name,
                                      user.id, &krate.description,
                                      &krate.homepage,
                                      &krate.documentation,
                                      &krate.readme,
                                      &krate.keywords,
                                      &krate.repository,
                                      &krate.license,
                                      &None).unwrap();
    Keyword::update_crate(req.tx().unwrap(), &krate,
                          &krate.keywords).unwrap();
    let v = krate.add_version(req.tx().unwrap(), v, &HashMap::new(), &[]);
    (krate, v.unwrap())
}

fn mock_dep(req: &mut Request, version: &Version, krate: &Crate,
            target: Option<&str>) -> Dependency {
    Dependency::insert(req.tx().unwrap(),
                       version.id,
                       krate.id,
                       &semver::VersionReq::parse(">= 0").unwrap(),
                       Kind::Normal,
                       false, true, &[],
                       &target.map(|s| s.to_string())).unwrap()
}

fn mock_keyword(req: &mut Request, name: &str) -> Keyword {
    Keyword::find_or_insert(req.tx().unwrap(), name).unwrap()
}

fn logout(req: &mut Request) {
    req.mut_extensions().pop::<User>();
}
