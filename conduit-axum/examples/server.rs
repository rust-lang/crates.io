#![deny(clippy::all)]

use axum::routing::get;
use conduit_axum::{server_error_response, ConduitAxumHandler, ConduitRequest, HandlerResult};

use axum::response::IntoResponse;
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

fn endpoint(_: ConduitRequest) -> HandlerResult {
    sleep(std::time::Duration::from_secs(2));

    "Hello world!".into_response()
}

fn panic(_: ConduitRequest) -> HandlerResult {
    // For now, connection is immediately closed
    panic!("message");
}

fn error(_: ConduitRequest) -> HandlerResult {
    server_error_response(&io::Error::new(io::ErrorKind::Other, "io error, oops"))
}
