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
//! use conduit_axum::{Handler, ConduitAxumHandler};
//! use tokio::runtime::Runtime;
//!
//! #[tokio::main]
//! async fn main() {
//!     let router = axum::Router::new()
//!         .route("/", get(ConduitAxumHandler::wrap(build_conduit_handler())));
//!
//!     let addr = ([127, 0, 0, 1], 12345).into();
//!
//!     axum::Server::bind(&addr)
//!         .serve( router.into_make_service())
//!         .await
//!         .unwrap();
//! }
//!
//! fn build_conduit_handler() -> impl Handler {
//!     // ...
//! #     Endpoint()
//! }
//! #
//! # use std::{error, io};
//! # use axum::body::Bytes;
//! # use conduit_axum::{box_error, Response, ConduitRequest, HandlerResult};
//! #
//! # struct Endpoint();
//! # impl Handler for Endpoint {
//! #     fn call(&self, _: &mut ConduitRequest) -> HandlerResult {
//! #         Response::builder().body(Bytes::new()).map_err(box_error)
//! #     }
//! # }
//! ```

mod body;
mod conduit;
mod error;
mod fallback;
mod response;
#[cfg(test)]
mod tests;
mod tokio_utils;

pub use conduit::*;
pub use error::ServiceError;
pub use fallback::{CauseField, ConduitAxumHandler, ErrorField, RequestParamsExt};
pub use response::conduit_into_axum;
pub use tokio_utils::spawn_blocking;
