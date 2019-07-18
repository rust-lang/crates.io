#![deny(warnings, clippy::all)]
#![feature(async_await)]

use conduit::{Handler, Request, Response};
use conduit_hyper::Server;
use conduit_router::RouteBuilder;
use futures::executor::block_on;
use tokio::runtime;

use std::collections::HashMap;
use std::io::{Cursor, Error};
use std::thread::sleep;

fn main() {
    let app = build_conduit_handler();
    let addr = ([127, 0, 0, 1], 12345).into();
    let server = Server::bind(&addr, app);

    let rt = runtime::Builder::new()
        // Set the max number of concurrent requests (tokio defaults to 100)
        .blocking_threads(2)
        .build()
        .unwrap();
    rt.spawn(async {
        server.await.unwrap();
    });
    block_on(rt.shutdown_on_idle());
}

fn build_conduit_handler() -> impl Handler {
    let mut router = RouteBuilder::new();
    router.get("/", endpoint);
    router.get("/panic", panic);
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
