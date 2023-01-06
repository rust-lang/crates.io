#![deny(clippy::all, missing_debug_implementations, rust_2018_idioms)]

//! A wrapper for integrating `hyper 0.13` with a `conduit 0.8` blocking application stack.
//!
//! A `conduit_axum::Handler` is allowed to block so the `Server` must be spawned on the (default)
//! multi-threaded `Runtime` which allows (by default) 100 concurrent blocking threads.  Any excess
//! requests will asynchronously await for an available blocking thread.
//!
//! # Examples
//!
//! Try out the example with `cargo run --example server`.
//!
//! Typical usage:
//!
//! ```no_run
//! use axum::routing::get;
//! use axum::response::IntoResponse;
//! use tokio::runtime::Runtime;
//!
//! #[tokio::main]
//! async fn main() {
//!     let router = axum::Router::new()
//!         .route("/", get(handler));
//!
//!     let addr = ([127, 0, 0, 1], 12345).into();
//!
//!     axum::Server::bind(&addr)
//!         .serve( router.into_make_service())
//!         .await
//!         .unwrap();
//! }
//!
//! async fn handler() -> impl IntoResponse {
//!     // ...
//! }
//! ```

mod conduit;
mod error;
mod fallback;
mod response;
#[cfg(test)]
mod tests;
mod tokio_utils;

pub use conduit::*;
pub use error::ServiceError;
pub use fallback::{server_error_response, CauseField, ErrorField};
pub use tokio_utils::spawn_blocking;
