#![deny(clippy::all)]

use conduit::{Handler, Request, Response};
use conduit_hyper::Server;
use conduit_router::RouteBuilder;

use std::collections::HashMap;
use std::io::{Cursor, Error};
use std::thread::sleep;

#[tokio::main]
async fn main() {
    env_logger::init();

    let app = build_conduit_handler();
    let addr = ([127, 0, 0, 1], 12345).into();

    // FIXME: Set limit on number of blocking tasks
    Server::serve(&addr, app).await;
}

fn build_conduit_handler() -> impl Handler {
    let mut router = RouteBuilder::new();
    router.get("/", endpoint);
    router.get("/panic", panic);
    router.get("/error", error);
    router
}

fn endpoint(_: &mut dyn Request) -> Result<Response, Error> {
    let body = "Hello world!";

    sleep(std::time::Duration::from_secs(2));

    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        vec!["text/plain; charset=utf-8".to_string()],
    );
    headers.insert("Content-Length".to_string(), vec![body.len().to_string()]);
    Ok(Response {
        status: (200, "OK"),
        headers,
        body: Box::new(Cursor::new(body)),
    })
}

fn panic(_: &mut dyn Request) -> Result<Response, Error> {
    // For now, connection is immediately closed
    panic!("message");
}

fn error(_: &mut dyn Request) -> Result<Response, Error> {
    Err(Error::new(std::io::ErrorKind::Other, "io error, oops"))
}
