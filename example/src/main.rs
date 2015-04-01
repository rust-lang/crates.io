extern crate civet;
extern crate conduit;
extern crate conduit_router;

use std::collections::HashMap;
use std::io::{self, Cursor};
use std::sync::mpsc::channel;

use civet::{Config, response, Server};
use conduit::{Request, Response};
use conduit_router::{RouteBuilder, RequestParams};

fn name(req: &mut Request) -> io::Result<Response> {
    let name = req.params().find("name").unwrap();
    let bytes = format!("Hello {}!", name).into_bytes();
    Ok(response(200, HashMap::new(), Cursor::new(bytes)))
}

fn hello(_req: &mut Request) -> io::Result<Response> {
    Ok(response(200, HashMap::new(), "Hello world!".as_bytes()))
}

fn main() {
    let mut router = RouteBuilder::new();

    router.get("/", hello);
    router.get("/:name", name);

    let _server = Server::start(Config { port: 8888, threads: 1 }, router);

    // Preventing process exit.
    let (_tx, rx) = channel::<()>();
    rx.recv().unwrap();
}
