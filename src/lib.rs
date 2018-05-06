extern crate conduit;
extern crate futures;
extern crate futures_cpupool;
extern crate http;
extern crate hyper;
extern crate semver;

use std::io::{Cursor, Read};
use std::net::SocketAddr;
use std::sync::Arc;

use futures::{future, Stream};
use futures_cpupool::CpuPool;
use hyper::rt::Future;
use hyper::{Body, Method, Request, Response, Server, StatusCode, Version};

#[derive(Debug)]
struct Parts(http::request::Parts);

impl conduit::Headers for Parts {
    /// Find all values associated with a header, or None.
    ///
    /// If the value of a header is not valid UTF-8, that value
    /// is replaced with the emtpy string.
    fn find(&self, key: &str) -> Option<Vec<&str>> {
        let values = self.0
            .headers
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
        self.0.headers.contains_key(key)
    }

    /// Returns a representation of all headers
    ///
    /// There is currently a bug where keys with mutliple values will be duplicated.
    /// See: https://github.com/hyperium/http/issues/199
    fn all(&self) -> Vec<(&str, Vec<&str>)> {
        let keys = self.0.headers.keys();
        let mut all = Vec::new();
        for key in keys {
            let key = key.as_str();
            let values = self.find(key)
                .expect("all keys should have at least one value");
            all.push((key, values));
        }
        all
    }
}

struct ConduitRequest<'a> {
    parts: Parts,
    body: Cursor<&'a [u8]>,
    extensions: conduit::Extensions,
}

impl<'a> conduit::Request for ConduitRequest<'a> {
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

    fn scheme(&self) -> conduit::Scheme {
        match self.parts.0.uri.scheme_part() {
            Some(s) if s.as_str() == "https" => conduit::Scheme::Https,
            _ => conduit::Scheme::Http,
        }
    }

    fn headers(&self) -> &conduit::Headers {
        &self.parts
    }

    fn content_length(&self) -> Option<u64> {
        Some(self.body.get_ref().len() as u64)
    }

    /// Always returns an address of 0.0.0.0:0
    fn remote_addr(&self) -> SocketAddr {
        ([0, 0, 0, 0], 0).into()
    }

    fn virtual_root(&self) -> Option<&str> {
        None
    }

    fn path(&self) -> &str {
        &self.parts.0.uri.path()
    }

    fn extensions(&self) -> &conduit::Extensions {
        &self.extensions
    }

    fn mut_extensions(&mut self) -> &mut conduit::Extensions {
        &mut self.extensions
    }

    fn host(&self) -> conduit::Host {
        // FIXME: Ensure the hyper server always provides a host so that unwrap is okay
        conduit::Host::Name(&self.parts.0.uri.host().unwrap())
    }

    fn query_string(&self) -> Option<&str> {
        self.parts.0.uri.query()
    }

    fn body(&mut self) -> &mut Read {
        self.body.set_position(0);
        &mut self.body
    }
}

impl<'a> ConduitRequest<'a> {
    fn new(parts: Parts, body: &'a [u8]) -> ConduitRequest<'a> {
        ConduitRequest {
            parts,
            body: Cursor::new(body),
            extensions: conduit::Extensions::new(),
        }
    }
}

pub struct Service<H> {
    pool: CpuPool,
    handler: Arc<H>,
}

// #[derive(Clone)] results in cloning a ref, and not the Service
impl<H> Clone for Service<H> {
    fn clone(&self) -> Self {
        Service {
            pool: self.pool.clone(),
            handler: self.handler.clone(),
        }
    }
}

impl<H: conduit::Handler> hyper::service::NewService for Service<H> {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Service = Service<H>;
    type Future = Box<Future<Item = Self::Service, Error = Self::InitError> + Send>;
    type InitError = hyper::Error;

    fn new_service(&self) -> Self::Future {
        Box::new(future::ok(self.clone()))
    }
}

impl<H: conduit::Handler> hyper::service::Service for Service<H> {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Response<Self::ResBody>, Error = Self::Error> + Send>;

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        let pool = self.pool.clone();
        let handler = self.handler.clone();

        let (parts, body) = request.into_parts();
        let response = body.concat2().and_then(move |full_body| {
            pool.spawn_fn(move || {
                let mut request = ConduitRequest::new(Parts(parts), &*full_body);
                let response = handler
                    .call(&mut request)
                    .map(good_response)
                    .unwrap_or_else(|e| error_response(e.description()));

                future::ok(response)
            })
        });
        Box::new(response)
    }
}

impl<H: conduit::Handler> Service<H> {
    pub fn new(handler: H, threads: usize) -> Service<H> {
        let pool = CpuPool::new(threads);
        Service {
            pool,
            handler: Arc::new(handler),
        }
    }

    pub fn run(&self, addr: SocketAddr) {
        let server = Server::bind(&addr).serve(self.clone());
        hyper::rt::run(server.map_err(|e| eprintln!("server error: {}", e)));
    }
}

fn good_response(mut response: conduit::Response) -> Response<Body> {
    let mut body = Vec::new();
    if let Err(_) = response.body.write_body(&mut body) {
        return error_response("Error writing body");
    }

    let mut builder = Response::builder();
    let status =
        StatusCode::from_u16(response.status.0 as u16).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    builder.status(status);

    for (key, values) in response.headers {
        for value in values {
            builder.header(key.as_str(), value.as_str());
        }
    }

    builder.body(body.into()).unwrap() // FIXME: unwrap
}

fn error_response(error: &str) -> Response<Body> {
    eprintln!("Internal Server Error: {}", error);
    let body = Body::from("Internal Server Error");
    Response::builder().status(500).body(body).unwrap() // FIXME: unwrap
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
