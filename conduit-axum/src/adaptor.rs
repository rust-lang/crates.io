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

use conduit::RequestExt;
use http::request::Parts as HttpParts;
use http::{Extensions, HeaderMap, Method, Request, Version};
use hyper::body::Bytes;

pub(crate) struct ConduitRequest {
    parts: HttpParts,
    path: String,
    body: Cursor<Bytes>,
}

impl ConduitRequest {
    pub(crate) fn new(request: Request<Bytes>) -> Self {
        let (parts, body) = request.into_parts();
        let path = parts.uri.path().as_bytes();
        let path = percent_encoding::percent_decode(path)
            .decode_utf8_lossy()
            .into_owned();

        Self {
            parts,
            path,
            body: Cursor::new(body),
        }
    }
}

impl RequestExt for ConduitRequest {
    fn http_version(&self) -> Version {
        self.parts.version
    }

    fn method(&self) -> &Method {
        &self.parts.method
    }

    fn headers(&self) -> &HeaderMap {
        &self.parts.headers
    }

    /// Returns the length of the buffered body
    fn content_length(&self) -> Option<u64> {
        Some(self.body.get_ref().len() as u64)
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn extensions(&self) -> &Extensions {
        &self.parts.extensions
    }

    fn mut_extensions(&mut self) -> &mut Extensions {
        &mut self.parts.extensions
    }

    fn query_string(&self) -> Option<&str> {
        self.parts.uri.query()
    }

    fn body(&mut self) -> &mut dyn Read {
        &mut self.body
    }
}
