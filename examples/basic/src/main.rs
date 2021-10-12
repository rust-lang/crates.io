extern crate civet;
extern crate conduit;
extern crate conduit_router;

use std::sync::mpsc::channel;

use civet::{Config, Server};
use conduit::{Body, HttpResult, RequestExt, Response};
use conduit_router::{RequestParams, RouteBuilder};

fn name(req: &mut dyn RequestExt) -> HttpResult {
    let name = req.params().find("name").unwrap();
    let bytes = format!("Hello {}!", name).into_bytes();
    Response::builder().body(Body::from_vec(bytes))
}

fn hello(_req: &mut dyn RequestExt) -> HttpResult {
    Response::builder().body(Body::from_static(b"Hello world!"))
}

fn main() {
    let mut router = RouteBuilder::new();

    router.get("/", hello);
    router.get("/:name", name);

    let mut cfg = Config::new();
    cfg.port(8888).threads(1);
    let _server = Server::start(cfg, router);

    // Preventing process exit.
    let (_tx, rx) = channel::<()>();
    rx.recv().unwrap();
}
