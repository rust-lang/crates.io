#![feature(macro_rules)]

extern crate green;
extern crate rustuv;
extern crate serialize;

extern crate civet;
extern crate curl;
extern crate html;
extern crate oauth2;
extern crate pg = "postgres";

extern crate conduit_router = "conduit-router";
extern crate conduit;
extern crate conduit_cookie = "conduit-cookie";
extern crate conduit_middleware = "conduit-middleware";

use std::io::{IoResult, MemReader, MemWriter};
use std::collections::HashMap;

use civet::{Config, Server, response};
use conduit::Response;
use conduit_router::RouteBuilder;
use conduit_middleware::MiddlewareBuilder;

use app::App;

mod app;
mod db;
mod packages;
mod user;
mod util;

fn main() {
    let mut router = RouteBuilder::new();
    router.get("/", packages::index);
    router.get("/packages/new", packages::new);
    router.post("/packages/new", packages::create);
    router.get("/packages/:id", packages::get);

    router.get("/users/auth/github/authorize", user::github_authorize);
    router.get("/users/auth/github", user::github_access_token);

    let mut m = MiddlewareBuilder::new(router);
    m.add(conduit_cookie::Middleware::new(b"application-key"));
    m.add(conduit_cookie::SessionMiddleware::new("cargo_session"));
    m.add(app::AppMiddleware::new(App::new()));
    m.add(user::Middleware);

    let port = 8888;
    let _a = Server::start(Config { port: port, threads: 8 }, m);
    println!("listening on port {}", port);
    wait_for_sigint();
}

fn layout(f: |&mut Writer| -> IoResult<()>) -> IoResult<Response> {
    let mut dst = MemWriter::new();
    try!(write!(&mut dst, r"
        <html>
            <head>
            </head>
            <body>"));
    try!(f(&mut dst));
    try!(write!(&mut dst, r"
            </body>
        </html>"));
    Ok(response(200i, HashMap::new(), MemReader::new(dst.unwrap())))
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
