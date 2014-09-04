#![feature(macro_rules)]

extern crate serialize;
extern crate url;
extern crate semver;

extern crate curl;
extern crate flate2;
extern crate html;
extern crate oauth2;
extern crate pg = "postgres";
extern crate r2d2;
extern crate r2d2_postgres;
extern crate s3;

extern crate conduit_router = "conduit-router";
extern crate conduit;
extern crate conduit_cookie = "conduit-cookie";
extern crate conduit_middleware = "conduit-middleware";
extern crate conduit_conditional_get = "conduit-conditional-get";
extern crate conduit_log_requests = "conduit-log-requests";
extern crate conduit_static = "conduit-static";
extern crate conduit_json_parser = "conduit-json-parser";

pub use config::Config;
pub use app::App;
pub use db::Pool;

use conduit_router::RouteBuilder;
use conduit_middleware::MiddlewareBuilder;

use util::C;

mod macros;

mod app;
mod config;
mod db;
mod dist;
mod git;
mod package;
mod user;
mod util;

#[deriving(PartialEq, Eq)]
pub enum Environment {
    Development,
    Test,
    Production,
}

pub fn middleware(app: App) -> MiddlewareBuilder {
    let mut router = RouteBuilder::new();

    router.get("/authorize_url", C(user::github_authorize));
    router.get("/authorize", C(user::github_access_token));
    router.get("/logout", C(user::logout));
    router.get("/me", C(user::me));
    router.put("/me/reset_token", C(user::reset_token));
    router.get("/packages", C(package::index));
    router.get("/packages/:package_id", C(package::show));
    router.put("/packages/:package_id", {
        let mut m = MiddlewareBuilder::new(C(package::update));
        m.add(conduit_json_parser::BodyReader::<package::UpdateRequest>);
        m
    });
    router.post("/packages/new", C(package::new));
    router.get("/git/index/*path", C(git::serve_index));
    router.post("/git/index/*path", C(git::serve_index));

    let mut m = MiddlewareBuilder::new(router);
    let env = app.env;
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
