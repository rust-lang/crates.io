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
use http::{Extensions, HeaderMap, Method, Request, Uri};
use hyper::body::Bytes;

pub(crate) struct ConduitRequest {
    request: Request<Cursor<Bytes>>,
}

impl ConduitRequest {
    pub(crate) fn new(request: Request<Cursor<Bytes>>) -> Self {
        Self { request }
    }
}

impl RequestExt for ConduitRequest {
    fn method(&self) -> &Method {
        self.request.method()
    }

    fn uri(&self) -> &Uri {
        self.request.uri()
    }

    fn headers(&self) -> &HeaderMap {
        self.request.headers()
    }

    /// Returns the length of the buffered body
    fn content_length(&self) -> Option<u64> {
        Some(self.request.body().get_ref().len() as u64)
    }

    fn extensions(&self) -> &Extensions {
        self.request.extensions()
    }

    fn extensions_mut(&mut self) -> &mut Extensions {
        self.request.extensions_mut()
    }

    fn body(&mut self) -> &mut dyn Read {
        self.request.body_mut()
    }
}
