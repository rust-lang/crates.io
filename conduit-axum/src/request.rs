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
use http::{Extensions, HeaderMap, Method, Request, Uri};
use hyper::body::Bytes;

pub(crate) struct ConduitRequest {
    parts: HttpParts,
    body: Cursor<Bytes>,
}

impl ConduitRequest {
    pub(crate) fn new(request: Request<Bytes>) -> Self {
        let (parts, body) = request.into_parts();

        Self {
            parts,
            body: Cursor::new(body),
        }
    }
}

impl RequestExt for ConduitRequest {
    fn method(&self) -> &Method {
        &self.parts.method
    }

    fn uri(&self) -> &Uri {
        &self.parts.uri
    }

    fn headers(&self) -> &HeaderMap {
        &self.parts.headers
    }

    /// Returns the length of the buffered body
    fn content_length(&self) -> Option<u64> {
        Some(self.body.get_ref().len() as u64)
    }

    fn extensions(&self) -> &Extensions {
        &self.parts.extensions
    }

    fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.parts.extensions
    }

    fn body(&mut self) -> &mut dyn Read {
        &mut self.body
    }
}
