use std::io::{Cursor, Read};
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};

use hyper::{Chunk, Method, Version};

/// Owned data consumed by the worker thread
///
/// `ConduitRequest` cannot be sent between threads, so the input data is
/// captured on a core thread and taken by the worker thread.
pub(crate) struct RequestInfo(Option<(Parts, Chunk)>);

impl RequestInfo {
    /// Save the request info that can be sent between threads
    pub(crate) fn new(parts: http::request::Parts, body: Chunk) -> Self {
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

#[derive(Debug)]
pub(crate) struct Parts(http::request::Parts);

impl Parts {
    fn headers(&self) -> &http::HeaderMap {
        &self.0.headers
    }
}

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

pub(crate) struct ConduitRequest {
    parts: Parts,
    path: String,
    remote_addr: SocketAddr,
    body: Cursor<Chunk>,
    extensions: conduit::Extensions, // makes struct non-Send
}

impl ConduitRequest {
    pub(crate) fn new(info: &mut RequestInfo, remote_addr: SocketAddr) -> Self {
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

fn version(major: u64, minor: u64) -> semver::Version {
    semver::Version {
        major,
        minor,
        patch: 0,
        pre: vec![],
        build: vec![],
    }
}
