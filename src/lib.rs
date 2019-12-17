#![deny(warnings, clippy::all, missing_debug_implementations, rust_2018_idioms)]
#![allow(clippy::needless_doctest_main, clippy::unknown_clippy_lints)] // using the tokio::main attribute

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
//! #[tokio::main]
//! async fn main() {
//!     let app = build_conduit_handler();
//!     let addr = ([127, 0, 0, 1], 12345).into();
//!     let server = Server::serve(&addr, app);
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
//! # use conduit::{Request, Response};
//! #
//! # struct Endpoint();
//! # impl Handler for Endpoint {
//! #     fn call(&self, _: &mut dyn Request) -> Result<Response, Box<dyn error::Error + Send>> {
//! #         Ok(Response {
//! #             status: (200, "OK"),
//! #             headers: Default::default(),
//! #             body: Box::new(io::Cursor::new("")),
//! #         })
//! #     }
//! # }
//! ```

mod adaptor;
mod server;
mod service;
#[cfg(test)]
mod tests;

// Consumers of this library need access to this particular version of `semver`
pub use semver;

pub use server::Server;
pub use service::Service;
