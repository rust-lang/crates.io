#![deny(clippy::all)]

use axum::routing::get;
use conduit_axum::{
    server_error_response, spawn_blocking, ConduitRequest, HandlerResult, ServiceError,
};

use axum::response::IntoResponse;
use std::io;
use std::thread::sleep;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let router = axum::Router::new()
        .route("/", get(endpoint))
        .route("/panic", get(panic))
        .route("/error", get(error));

    let addr = ([127, 0, 0, 1], 12345).into();

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap()
}

async fn endpoint(_: ConduitRequest) -> HandlerResult {
    spawn_blocking(move || sleep(std::time::Duration::from_secs(2)))
        .await
        .map_err(ServiceError::from)
        .map(|_| "Hello world!")
        .into_response()
}

async fn panic(_: ConduitRequest) -> HandlerResult {
    // For now, connection is immediately closed
    panic!("message");
}

async fn error(_: ConduitRequest) -> HandlerResult {
    server_error_response(&io::Error::new(io::ErrorKind::Other, "io error, oops"))
}
