#![feature(macro_rules)]

extern crate serialize;
extern crate time;

extern crate "postgres" as pg;
extern crate curl;
extern crate flate2;
extern crate git2;
extern crate html;
extern crate oauth2;
extern crate openssl;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate s3;
extern crate semver;
extern crate url;

extern crate "conduit-router" as conduit_router;
extern crate conduit;
extern crate "conduit-cookie" as conduit_cookie;
extern crate "conduit-middleware" as conduit_middleware;
extern crate "conduit-conditional-get" as conduit_conditional_get;
extern crate "conduit-log-requests" as conduit_log_requests;
extern crate "conduit-static" as conduit_static;
extern crate "conduit-json-parser" as conduit_json_parser;

pub use config::Config;
pub use app::App;

use std::sync::Arc;

use conduit_router::RouteBuilder;
use conduit_middleware::MiddlewareBuilder;

use util::C;

mod macros;

pub mod app;
pub mod config;
pub mod db;
pub mod dependency;
pub mod dist;
pub mod git;
pub mod package;
pub mod user;
pub mod util;
pub mod version;

#[deriving(PartialEq, Eq, Clone)]
pub enum Environment {
    Development,
    Test,
    Production,
}

pub fn middleware(app: Arc<App>) -> MiddlewareBuilder {
    let mut router = RouteBuilder::new();

    router.get("/authorize_url", C(user::github_authorize));
    router.get("/authorize", C(user::github_access_token));
    router.get("/logout", C(user::logout));
    router.get("/me", C(user::me));
    router.put("/me/reset_token", C(user::reset_token));
    router.get("/packages", C(package::index));
    router.get("/versions", C(version::index));
    router.get("/versions/:version_id", C(version::show));
    router.get("/packages/:package_id", C(package::show));
    router.put("/packages/:package_id", {
        let mut m = MiddlewareBuilder::new(C(package::update));
        m.add(conduit_json_parser::BodyReader::<package::UpdateRequest>);
        m
    });
    router.put("/packages/new", C(package::new));
    router.get("/git/index/*path", C(git::serve_index));
    router.post("/git/index/*path", C(git::serve_index));

    let mut m = MiddlewareBuilder::new(router);
    let env = app.config.env;
    if env != Test {
        m.add(conduit_log_requests::LogRequests(0));
    }
    m.add(conduit_conditional_get::ConditionalGet);
    m.add(conduit_cookie::Middleware::new(app.session_key.as_bytes()));
    m.add(conduit_cookie::SessionMiddleware::new("cargo_session"));
    m.add(app::AppMiddleware::new(app));
    m.add(db::TransactionMiddleware);
    m.add(user::Middleware);
    if env != Test {
        m.around(dist::Middleware::new());
    }
    return m;
}

pub fn now() -> time::Timespec {
    time::now_utc().to_timespec()
}

pub fn encode_time(ts: time::Timespec) -> String {
    time::at_utc(ts).rfc3339()
}
