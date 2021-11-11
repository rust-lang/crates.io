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

use conduit::{Extensions, HeaderMap, Host, Method, RequestExt, Scheme, StartInstant, Version};
use http::request::Parts as HttpParts;
use http::Request;
use hyper::body::Bytes;

pub(crate) struct ConduitRequest {
    parts: HttpParts,
    path: String,
    remote_addr: SocketAddr,
    body: Cursor<Bytes>,
}

impl ConduitRequest {
    pub(crate) fn new(request: Request<Bytes>, remote_addr: SocketAddr, now: StartInstant) -> Self {
        let (mut parts, body) = request.into_parts();
        let path = parts.uri.path().as_bytes();
        let path = percent_encoding::percent_decode(path)
            .decode_utf8_lossy()
            .into_owned();

        parts.extensions.insert(now);

        Self {
            parts,
            path,
            remote_addr,
            body: Cursor::new(body),
        }
    }

    fn parts(&self) -> &HttpParts {
        &self.parts
    }
}

impl RequestExt for ConduitRequest {
    fn http_version(&self) -> Version {
        self.parts().version
    }

    fn method(&self) -> &Method {
        &self.parts().method
    }

    /// Always returns Http
    fn scheme(&self) -> Scheme {
        Scheme::Http
    }

    fn headers(&self) -> &HeaderMap {
        &self.parts.headers
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

    fn path_mut(&mut self) -> &mut String {
        &mut self.path
    }

    fn extensions(&self) -> &Extensions {
        &self.parts.extensions
    }

    fn mut_extensions(&mut self) -> &mut Extensions {
        &mut self.parts.extensions
    }

    /// Returns the value of the `Host` header
    ///
    /// If the header is not present or is invalid UTF-8, then the empty string is returned
    fn host(&self) -> Host<'_> {
        let host = self
            .headers()
            .get(http::header::HOST)
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
