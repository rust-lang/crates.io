#![deny(clippy::all, missing_debug_implementations, rust_2018_idioms)]

//! A wrapper for integrating `hyper 0.13` with a `conduit 0.8` blocking application stack.
//!
//! A `conduit::Handler` is allowed to block so the `Server` must be spawned on the (default)
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
//! use conduit::Handler;
//! use conduit_hyper::Server;
//! use tokio::runtime::Runtime;
//!
//! const MAX_THREADS: usize = 10;
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = build_conduit_handler();
//!     let addr = ([127, 0, 0, 1], 12345).into();
//!     let server = Server::serve(&addr, app, MAX_THREADS);
//!
//!     server.await;
//! }
//!
//! fn build_conduit_handler() -> impl Handler {
//!     // ...
//! #     Endpoint()
//! }
//! #
//! # use std::{error, io};
//! # use conduit::{box_error, Body, Response, RequestExt, HandlerResult};
//! #
//! # struct Endpoint();
//! # impl Handler for Endpoint {
//! #     fn call(&self, _: &mut dyn RequestExt) -> HandlerResult {
//! #         Response::builder().body(Body::empty()).map_err(box_error)
//! #     }
//! # }
//! ```

mod adaptor;
mod file_stream;
mod server;
mod service;
#[cfg(test)]
mod tests;

pub use server::Server;
pub use service::{BlockingHandler, Service};

type HyperResponse = hyper::Response<hyper::Body>;
type ConduitResponse = conduit::Response<conduit::Body>;
