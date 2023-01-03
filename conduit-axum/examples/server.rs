#![deny(clippy::all)]

use axum::routing::get;
use conduit::{Body, RequestExt, ResponseResult};
use conduit_axum::ConduitAxumHandler;
use http::{header, Response};

use std::io;
use std::thread::sleep;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let router = axum::Router::new()
        .route("/", get(wrap(endpoint)))
        .route("/panic", get(wrap(panic)))
        .route("/error", get(wrap(error)));

    let addr = ([127, 0, 0, 1], 12345).into();

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap()
}

pub fn wrap<H>(handler: H) -> ConduitAxumHandler<H> {
    ConduitAxumHandler::wrap(handler)
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
