#![feature(macro_rules)]

extern crate "cargo-registry" as cargo_registry;
extern crate "conduit-test" as conduit_test;
extern crate "conduit-middleware" as conduit_middleware;
extern crate conduit;
extern crate serialize;

use std::sync::{Once, ONCE_INIT};
use serialize::json;

macro_rules! t( ($e:expr) => (
    match $e {
        Ok(e) => e,
        Err(m) => fail!(concat!(stringify!($e), " failed with: {}"), m),
    }
) )

macro_rules! t_resp( ($e:expr) => ({
    let resp = t!($e.map_err(|e| (&*e).to_string()));
    if !::ok_resp(&resp) { fail!("bad response: {}", resp.status); }
    resp
}) )

mod user;

fn app() -> cargo_registry::App {
    static mut INIT: Once = ONCE_INIT;

    let config = cargo_registry::Config {
        s3_bucket: "".to_string(),
        s3_access_key: "".to_string(),
        s3_secret_key: "".to_string(),
        session_key: "test".to_string(),
        git_repo_bare: Path::new("/"),
        git_repo_checkout: Path::new("/"),
        gh_client_id: "".to_string(),
        gh_client_secret: "".to_string(),
        db_url: env("TEST_DATABASE_URL"),
        env: cargo_registry::Test,
    };
    let app = cargo_registry::App::new(&config);
    unsafe {
        INIT.doit(|| app.db_setup());
    }
    return app;

    fn env(s: &str) -> String {
        match std::os::getenv(s) {
            Some(s) => s,
            None => fail!("must have `{}` defined", s),
        }
    }
}

fn middleware() -> conduit_middleware::MiddlewareBuilder {
    cargo_registry::middleware(app())
}

fn ok_resp(r: &conduit::Response) -> bool {
    r.status.val0() == 200
}

fn json<T>(r: &mut conduit::Response) -> T
           where T: serialize::Decodable<json::Decoder, json::DecoderError> {
    let data = r.body.read_to_end().unwrap();
    let s = std::str::from_utf8(data.as_slice()).unwrap();
    json::decode(s).unwrap()
}
