#![feature(macro_rules)]

extern crate "cargo-registry" as cargo_registry;
extern crate "conduit-middleware" as conduit_middleware;
extern crate "conduit-test" as conduit_test;
extern crate conduit;
extern crate curl;
extern crate git2;
extern crate serialize;
extern crate url;

use std::sync::{Once, ONCE_INIT, Arc};
use std::os;
use serialize::json;

use conduit::Request;
use conduit_test::MockRequest;
use cargo_registry::app::App;
use cargo_registry::db;
use cargo_registry::user::User;

macro_rules! t( ($e:expr) => (
    match $e {
        Ok(e) => e,
        Err(m) => fail!(concat!(stringify!($e), " failed with: {}"), m),
    }
) )

macro_rules! t_resp( ($e:expr) => ({
    t!($e.map_err(|e| (&*e).to_string()))
}) )

macro_rules! ok_resp( ($e:expr) => ({
    let resp = t_resp!($e);
    if !::ok_resp(&resp) { fail!("bad response: {}", resp.status); }
    resp
}) )

mod middleware;
mod package;
mod user;
mod record;
mod git;
mod version;

fn app() -> (record::Bomb, Arc<App>, conduit_middleware::MiddlewareBuilder) {
    static mut INIT: Once = ONCE_INIT;

    let (proxy, bomb) = record::proxy();
    let config = cargo_registry::Config {
        s3_bucket: os::getenv("S3_BUCKET").unwrap_or(String::new()),
        s3_access_key: os::getenv("S3_ACCESS_KEY").unwrap_or(String::new()),
        s3_secret_key: os::getenv("S3_SECRET_KEY").unwrap_or(String::new()),
        s3_proxy: Some(proxy),
        session_key: "test".to_string(),
        git_repo_bare: git::bare(),
        git_repo_checkout: git::checkout(),
        gh_client_id: "".to_string(),
        gh_client_secret: "".to_string(),
        db_url: env("TEST_DATABASE_URL"),
        env: cargo_registry::Test,
        max_upload_size: 100,
    };
    let app = App::new(&config);
    git::init();
    unsafe {
        INIT.doit(|| app.db_setup());
    }
    let app = Arc::new(app);
    return (bomb, app.clone(), cargo_registry::middleware(app));

    fn env(s: &str) -> String {
        match os::getenv(s) {
            Some(s) => s,
            None => fail!("must have `{}` defined", s),
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

fn json<T>(r: &mut conduit::Response) -> T
           where T: serialize::Decodable<json::Decoder, json::DecoderError> {
    let data = r.body.read_to_end().unwrap();
    let s = std::str::from_utf8(data.as_slice()).unwrap();
    match json::decode(s) {
        Ok(t) => t,
        Err(e) => fail!("failed to decode: {}\n{}", e, s),
    }
}

fn user() -> User {
    User {
        id: 10000,
        email: "foo@example.com".to_string(),
        gh_access_token: User::new_api_token(), // just randomize it
        api_token: User::new_api_token(),
    }
}

fn package() -> cargo_registry::package::Package {
    cargo_registry::package::Package {
        id: 10000,
        name: "foo".to_string(),
        user_id: 100,
    }
}
