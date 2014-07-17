#![feature(macro_rules)]

extern crate green;
extern crate rustuv;
extern crate serialize;
extern crate url;
extern crate semver;

extern crate civet;
extern crate curl;
extern crate html;
extern crate oauth2;
extern crate pg = "postgres";
extern crate s3;

extern crate conduit_router = "conduit-router";
extern crate conduit;
extern crate conduit_cookie = "conduit-cookie";
extern crate conduit_middleware = "conduit-middleware";
extern crate conduit_conditional_get = "conduit-conditional-get";
extern crate conduit_log_requests = "conduit-log-requests";
extern crate conduit_static = "conduit-static";
extern crate conduit_json_parser = "conduit-json-parser";

use civet::{Config, Server};
use conduit_router::RouteBuilder;
use conduit_middleware::MiddlewareBuilder;

use app::App;
use util::C;

mod macros;

mod app;
mod db;
mod dist;
mod git;
mod package;
mod user;
mod util;

fn main() {
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
    router.post("/packages/new", {
        let mut m = MiddlewareBuilder::new(C(package::new));
        m.add(conduit_json_parser::BodyReader::<package::NewRequest>);
        m
    });
    router.get("/git/index/*path", C(git::serve_index));
    router.post("/git/index/*path", C(git::serve_index));

    let app = App::new();

    let mut m = MiddlewareBuilder::new(router);
    m.add(conduit_log_requests::LogRequests(0));
    m.add(conduit_conditional_get::ConditionalGet);
    m.add(conduit_cookie::Middleware::new(app.session_key.as_bytes()));
    m.add(conduit_cookie::SessionMiddleware::new("cargo_session"));
    m.add(app::AppMiddleware::new(app));
    m.add(user::Middleware);
    m.around(dist::Middleware::new());

    let port = 8888;
    let _a = Server::start(Config { port: port, threads: 8 }, m);
    println!("listening on port {}", port);
    wait_for_sigint();
}

// libnative doesn't have signal handling yet
fn wait_for_sigint() {
    use green::{SchedPool, PoolConfig, GreenTaskBuilder};
    use std::io::signal::{Listener, Interrupt};
    use std::task::TaskBuilder;

    let mut config = PoolConfig::new();
    config.event_loop_factory = rustuv::event_loop;

    let mut pool = SchedPool::new(config);
    TaskBuilder::new().green(&mut pool).spawn(proc() {
        let mut l = Listener::new();
        l.register(Interrupt).unwrap();
        l.rx.recv();
    });
    pool.shutdown();
}
