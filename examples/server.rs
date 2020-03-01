#![deny(clippy::all)]

use conduit::{header, Body, Handler, RequestExt, Response, ResponseResult};
use conduit_hyper::Server;
use conduit_router::RouteBuilder;

use std::io;
use std::thread::sleep;

const MAX_THREADS: usize = 1;

#[tokio::main]
async fn main() {
    env_logger::init();

    let app = build_conduit_handler();
    let addr = ([127, 0, 0, 1], 12345).into();

    Server::serve(&addr, app, MAX_THREADS).await;
}

fn build_conduit_handler() -> impl Handler {
    let mut router = RouteBuilder::new();
    router.get("/", endpoint);
    router.get("/panic", panic);
    router.get("/error", error);
    router
}

fn endpoint(_: &mut dyn RequestExt) -> ResponseResult<http::Error> {
    let body = b"Hello world!";

    sleep(std::time::Duration::from_secs(2));

    Response::builder()
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .header(header::CONTENT_LENGTH, body.len())
        .body(Body::from_static(body))
}

fn panic(_: &mut dyn RequestExt) -> ResponseResult<http::Error> {
    // For now, connection is immediately closed
    panic!("message");
}

fn error(_: &mut dyn RequestExt) -> ResponseResult<io::Error> {
    Err(io::Error::new(io::ErrorKind::Other, "io error, oops"))
}
