#![feature(async_await)]
#![deny(warnings, clippy::all, missing_debug_implementations)]

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
//! #![feature(async_await)]
//!
//! use conduit::Handler;
//! use conduit_hyper::Server;
//! use futures::executor::block_on;
//! use tokio::runtime::Runtime;
//!
//! #[hyper::rt::main]
//! async fn main() {
//!     let app = build_conduit_handler();
//!     let addr = ([127, 0, 0, 1], 12345).into();
//!     let server = Server::bind(&addr, app);
//!
//!     server.await;
//! }
//!
//! fn build_conduit_handler() -> impl Handler {
//!     // ...
//! #     Endpoint()
//! }
//! #
//! # use std::{collections, error, io};
//! #
//! # use conduit::{Request, Response};
//! #
//! # struct Endpoint();
//! #
//! # impl Handler for Endpoint {
//! #     fn call(&self, _: &mut dyn Request) -> Result<Response, Box<dyn error::Error + Send>> {
//! #         Ok(Response {
//! #             status: (200, "OK"),
//! #             headers: collections::HashMap::new(),
//! #             body: Box::new(io::Cursor::new("")),
//! #         })
//! #     }
//! # }
//! ```

#[cfg(test)]
mod tests;

use std::future::Future;
use std::io::{Cursor, Read};
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use futures::prelude::*;
use hyper::{Body, Chunk, Method, Request, Response, StatusCode, Version};
use log::error;

// Consumers of this library need access to this particular version of `semver`
pub use semver;

/// A builder for a `hyper::Server`
#[derive(Debug)]
pub struct Server;

impl Server {
    /// Bind a handler to an address
    pub fn bind<H: conduit::Handler>(addr: &SocketAddr, handler: H) -> impl Future {
        use hyper::server::conn::AddrStream;
        use hyper::service::{make_service_fn, service_fn};

        let handler = Arc::new(handler);

        let make_service = make_service_fn(move |socket: &AddrStream| {
            let handler = handler.clone();
            let remote_addr = socket.remote_addr();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |request: Request<Body>| {
                    let handler = handler.clone();

                    blocking_handler(handler, request, remote_addr)
                }))
            }
        });

        hyper::Server::bind(&addr).serve(make_service)
    }
}

async fn blocking_handler<H: conduit::Handler>(
    handler: Arc<H>,
    request: Request<Body>,
    remote_addr: std::net::SocketAddr,
) -> Result<Response<Body>, hyper::Error> {
    let (parts, body) = request.into_parts();

    body.try_concat()
        .and_then(|full_body| {
            let mut request_info = RequestInfo::new(parts, full_body);

            future::poll_fn(move |_| {
                tokio_threadpool::blocking(|| {
                    let mut request = ConduitRequest::new(&mut request_info, remote_addr);
                    handler
                        .call(&mut request)
                        .map(good_response)
                        .unwrap_or_else(|e| error_response(&e.to_string()))
                })
                .map_err(|_| panic!("the threadpool shut down"))
            })
        })
        .await
}

#[derive(Debug)]
struct Parts(http::request::Parts);

impl conduit::Headers for Parts {
    /// Find all values associated with a header, or None.
    ///
    /// If the value of a header is not valid UTF-8, that value
    /// is replaced with the emtpy string.
    fn find(&self, key: &str) -> Option<Vec<&str>> {
        let values = self
            .headers()
            .get_all(key)
            .iter()
            .map(|v| v.to_str().unwrap_or(""))
            .collect::<Vec<&str>>();

        if values.is_empty() {
            None
        } else {
            Some(values)
        }
    }

    fn has(&self, key: &str) -> bool {
        self.headers().contains_key(key)
    }

    /// Returns a representation of all headers
    fn all(&self) -> Vec<(&str, Vec<&str>)> {
        let mut all = Vec::new();
        for key in self.headers().keys() {
            let key = key.as_str();
            let values = self
                .find(key)
                .expect("all keys should have at least one value");
            all.push((key, values));
        }
        all
    }
}

impl Parts {
    fn headers(&self) -> &http::HeaderMap {
        &self.0.headers
    }
}

struct ConduitRequest {
    parts: Parts,
    path: String,
    remote_addr: SocketAddr,
    body: Cursor<Chunk>,
    extensions: conduit::Extensions, // makes struct non-Send
}

impl conduit::Request for ConduitRequest {
    fn http_version(&self) -> semver::Version {
        match self.parts.0.version {
            Version::HTTP_09 => version(0, 9),
            Version::HTTP_10 => version(1, 0),
            Version::HTTP_11 => version(1, 1),
            Version::HTTP_2 => version(2, 0),
        }
    }

    fn conduit_version(&self) -> semver::Version {
        version(0, 1)
    }

    fn method(&self) -> conduit::Method {
        match self.parts.0.method {
            Method::CONNECT => conduit::Method::Connect,
            Method::DELETE => conduit::Method::Delete,
            Method::GET => conduit::Method::Get,
            Method::HEAD => conduit::Method::Head,
            Method::OPTIONS => conduit::Method::Options,
            Method::PATCH => conduit::Method::Patch,
            Method::POST => conduit::Method::Post,
            Method::PUT => conduit::Method::Put,
            Method::TRACE => conduit::Method::Trace,
            _ => conduit::Method::Other(self.parts.0.method.to_string()),
        }
    }

    /// Always returns Http
    fn scheme(&self) -> conduit::Scheme {
        conduit::Scheme::Http
    }

    fn headers(&self) -> &dyn conduit::Headers {
        &self.parts
    }

    /// Returns the length of the buffered body
    fn content_length(&self) -> Option<u64> {
        Some(self.body.get_ref().len() as u64)
    }

    /// Always returns an address of 0.0.0.0:0
    fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    fn virtual_root(&self) -> Option<&str> {
        None
    }

    fn path(&self) -> &str {
        &*self.path
    }

    fn extensions(&self) -> &conduit::Extensions {
        &self.extensions
    }

    fn mut_extensions(&mut self) -> &mut conduit::Extensions {
        &mut self.extensions
    }

    /// Returns the value of the `Host` header
    ///
    /// If the header is not present or is invalid UTF-8, then the empty string is returned
    fn host(&self) -> conduit::Host<'_> {
        let host = self
            .parts
            .headers()
            .get("host")
            .map(|h| h.to_str().unwrap_or(""))
            .unwrap_or("");
        conduit::Host::Name(host)
    }

    fn query_string(&self) -> Option<&str> {
        self.parts.0.uri.query()
    }

    fn body(&mut self) -> &mut dyn Read {
        &mut self.body
    }
}

/// Owned data consumed by the worker thread
///
/// `ConduitRequest` cannot be sent between threads, so the input data is
/// captured on a core thread and taken by the worker thread.
struct RequestInfo(Option<(Parts, Chunk)>);

impl RequestInfo {
    /// Save the request info that can be sent between threads
    fn new(parts: http::request::Parts, body: Chunk) -> Self {
        let tuple = (Parts(parts), body);
        Self(Some(tuple))
    }

    /// Take back the request info
    ///
    /// Call this from the worker thread to obtain ownership of the `Send` data
    ///
    /// # Panics
    ///
    /// Panics if called more than once on a value
    fn take(&mut self) -> (Parts, Chunk) {
        self.0.take().expect("called take multiple times")
    }
}

impl ConduitRequest {
    fn new(info: &mut RequestInfo, remote_addr: SocketAddr) -> Self {
        let (parts, body) = info.take();
        let path = parts.0.uri.path().to_string();
        let path = Path::new(&path);
        let path = path
            .components()
            .fold(PathBuf::new(), |mut result, p| match p {
                Component::Normal(x) => {
                    if x != "" {
                        result.push(x)
                    };
                    result
                }
                Component::ParentDir => {
                    result.pop();
                    result
                }
                Component::RootDir => {
                    result.push(Component::RootDir);
                    result
                }
                _ => result,
            })
            .to_string_lossy()
            .to_string(); // non-Unicode is replaced with U+FFFD REPLACEMENT CHARACTER

        Self {
            parts,
            path,
            remote_addr,
            body: Cursor::new(body),
            extensions: conduit::Extensions::new(),
        }
    }
}

/// Builds a `hyper::Response` given a `conduit:Response`
fn good_response(mut response: conduit::Response) -> Response<Body> {
    let mut body = Vec::new();
    if response.body.write_body(&mut body).is_err() {
        return error_response("Error writing body");
    }

    let mut builder = Response::builder();
    let status = match StatusCode::from_u16(response.status.0 as u16) {
        Ok(s) => s,
        Err(e) => return error_response(&e.to_string()),
    };
    builder.status(status);

    for (key, values) in response.headers {
        for value in values {
            builder.header(key.as_str(), value.as_str());
        }
    }

    builder
        .body(body.into())
        .unwrap_or_else(|e| error_response(&e.to_string()))
}

/// Logs an error message and returns a generic status 500 response
fn error_response(message: &str) -> Response<Body> {
    error!("Internal Server Error: {}", message);
    let body = Body::from("Internal Server Error");
    Response::builder()
        .status(500)
        .body(body)
        .expect("unexpected invalid header")
}

fn version(major: u64, minor: u64) -> semver::Version {
    semver::Version {
        major,
        minor,
        patch: 0,
        pre: vec![],
        build: vec![],
    }
}
