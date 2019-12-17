//! Types implementing `conduit::Request` and `conduit::Headers` to provide to the guest application
//!
//! `ConduitRequest` and `Parts` implement `conduit::Request` and `conduit::Headers` respectively.
//! `Parts` is the concrete type that is returned from `ConduitRequest::headers()` as a
//! `&dyn conduit::Headers`.
//!
//! Because a `ConduitRequest` needs to carry around an `Extensions`, it cannot be `Send`.
//! Therefore, construction of this value must be deferred to the background thread where it will
//! be used.  To work around this, the essential request information from hyper is captured in a
//! `RequestInfo` which is `Send` and is moved into `ConduitRequest::new`.

use std::io::{Cursor, Read};
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};

use conduit::{Extensions, Headers, Host, Method, Request, Scheme};
use http::{request::Parts as HttpParts, HeaderMap};
use hyper::{body::Bytes, Method as HyperMethod, Version as HttpVersion};
use semver::Version;

/// Owned data consumed by the background thread
///
/// `ConduitRequest` cannot be sent between threads, so the needed request data
/// is extracted from hyper on a core thread and taken by the background thread.
pub(crate) struct RequestInfo(Option<(Parts, Bytes)>);

impl RequestInfo {
    /// Save the request info that can be sent between threads
    pub(crate) fn new(parts: HttpParts, body: Bytes) -> Self {
        let tuple = (Parts(parts), body);
        Self(Some(tuple))
    }

    /// Take back the request info
    ///
    /// Call this from the background thread to obtain ownership of the `Send` data.
    ///
    /// # Panics
    ///
    /// Panics if called more than once on a value.
    fn take(&mut self) -> (Parts, Bytes) {
        self.0.take().expect("called take multiple times")
    }
}

#[derive(Debug)]
pub(crate) struct Parts(HttpParts);

impl Parts {
    fn headers(&self) -> &HeaderMap {
        &self.0.headers
    }
}

impl Headers for Parts {
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
            .collect::<Vec<_>>();

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

pub(crate) struct ConduitRequest {
    parts: Parts,
    path: String,
    remote_addr: SocketAddr,
    body: Cursor<Bytes>,
    extensions: Extensions, // makes struct non-Send
}

impl ConduitRequest {
    pub(crate) fn new(info: &mut RequestInfo, remote_addr: SocketAddr) -> Self {
        let (parts, body) = info.take();
        let path = parts.0.uri.path().to_string();
        let path = Path::new(&path);
        let path = path
            .components()
            // Normalize path (needed by crates.io)
            // TODO: Make this optional?
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
            extensions: Extensions::new(),
        }
    }

    fn parts(&self) -> &HttpParts {
        &self.parts.0
    }
}

impl Request for ConduitRequest {
    fn http_version(&self) -> Version {
        match self.parts().version {
            HttpVersion::HTTP_09 => version(0, 9),
            HttpVersion::HTTP_10 => version(1, 0),
            HttpVersion::HTTP_11 => version(1, 1),
            HttpVersion::HTTP_2 => version(2, 0),
            HttpVersion::HTTP_3 => version(3, 0),
            _ => version(0, 0),
        }
    }

    fn conduit_version(&self) -> Version {
        version(0, 1)
    }

    fn method(&self) -> Method {
        match self.parts().method {
            HyperMethod::CONNECT => Method::Connect,
            HyperMethod::DELETE => Method::Delete,
            HyperMethod::GET => Method::Get,
            HyperMethod::HEAD => Method::Head,
            HyperMethod::OPTIONS => Method::Options,
            HyperMethod::PATCH => Method::Patch,
            HyperMethod::POST => Method::Post,
            HyperMethod::PUT => Method::Put,
            HyperMethod::TRACE => Method::Trace,
            _ => Method::Other(self.parts().method.to_string()),
        }
    }

    /// Always returns Http
    fn scheme(&self) -> Scheme {
        Scheme::Http
    }

    fn headers(&self) -> &dyn Headers {
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

    fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    fn mut_extensions(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    /// Returns the value of the `Host` header
    ///
    /// If the header is not present or is invalid UTF-8, then the empty string is returned
    fn host(&self) -> Host<'_> {
        let host = self
            .parts
            .headers()
            .get("host")
            .map(|h| h.to_str().unwrap_or(""))
            .unwrap_or("");
        Host::Name(host)
    }

    fn query_string(&self) -> Option<&str> {
        self.parts().uri.query()
    }

    fn body(&mut self) -> &mut dyn Read {
        &mut self.body
    }
}

fn version(major: u64, minor: u64) -> Version {
    Version {
        major,
        minor,
        patch: 0,
        pre: vec![],
        build: vec![],
    }
}
