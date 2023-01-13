#![deny(clippy::all)]

use axum::routing::get;
use conduit_axum::server_error_response;

use axum::response::IntoResponse;
use http::StatusCode;
use std::io;
use std::thread::sleep;
use tokio::task::spawn_blocking;

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

async fn endpoint() -> impl IntoResponse {
    spawn_blocking(move || sleep(std::time::Duration::from_secs(2)))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .map(|_| "Hello world!")
}

async fn panic() -> impl IntoResponse {
    // For now, connection is immediately closed
    panic!("message");
}

async fn error() -> impl IntoResponse {
    server_error_response(&io::Error::new(io::ErrorKind::Other, "io error, oops"))
}
