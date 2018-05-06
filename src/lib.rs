extern crate conduit;
extern crate futures;
extern crate futures_cpupool;
extern crate http;
extern crate hyper;
extern crate semver;

use std::io::{Cursor, Read};
use std::net::SocketAddr;

use futures::{future, Stream};
use futures_cpupool::CpuPool;
use hyper::rt::Future;
use hyper::{Body, Method, Request, Response, Server, Version};

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

#[derive(Clone)]
pub struct Service {
    pool: CpuPool,
}

impl hyper::service::NewService for Service {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Service = Service;
    type Future = Box<Future<Item=Self::Service, Error=Self::InitError> + Send>;
    type InitError = hyper::Error;

    fn new_service(&self) -> Self::Future {
        Box::new(future::ok(self.clone()))
    }
}

impl hyper::service::Service for Service {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response<Self::ResBody>, Error=Self::Error> + Send>;

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        let pool = self.pool.clone();

        let (parts, body) = request.into_parts();
        let future = body.concat2().and_then(|full_body| {
            let request = ConduitRequest::new(Parts(parts), &*full_body);
            future::ok("Hello, world!") // FIXME
        });
        let response = Response::new(Body::wrap_stream(future.into_stream()));
        Box::new(future::ok(response))
    }

}

impl Service {
    pub fn new(threads: usize) -> Service {
        let pool = CpuPool::new(threads);
        Service { pool }
    }

    pub fn run(&self, addr: SocketAddr) {
        let server = Server::bind(&addr).serve(self.clone());
        hyper::rt::run(server.map_err(|_| ()));
    }
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
