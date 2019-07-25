//! A helper that wraps a request and can overwrite either the path or the method.

use std::{io::Read, net::SocketAddr};

use conduit::{Method, Request};
use conduit_hyper::semver;

type RequestMutRef<'a> = &'a mut (dyn Request + 'a);

// Can't derive Debug because of Request.
#[allow(missing_debug_implementations)]
pub struct RequestProxy<'a> {
    other: RequestMutRef<'a>,
    path: Option<&'a str>,
    method: Option<conduit::Method>,
}

impl<'a> RequestProxy<'a> {
    /// Wrap a request and overwrite the path with the provided value.
    pub(crate) fn rewrite_path(req: RequestMutRef<'a>, path: &'a str) -> Self {
        RequestProxy {
            other: req,
            path: Some(path),
            method: None, // Defer to original request
        }
    }

    /// Wrap a request and overwrite the method with the provided value.
    pub(crate) fn rewrite_method(req: RequestMutRef<'a>, method: Method) -> Self {
        RequestProxy {
            other: req,
            path: None, // Defer to original request
            method: Some(method),
        }
    }
}

impl<'a> Request for RequestProxy<'a> {
    // Use local value if available, defer to the original request
    fn method(&self) -> conduit::Method {
        self.method.clone().unwrap_or_else(|| self.other.method())
    }

    fn path(&self) -> &str {
        self.path.unwrap_or_else(|| self.other.path())
    }

    // Pass-through
    fn http_version(&self) -> semver::Version {
        self.other.http_version()
    }
    fn conduit_version(&self) -> semver::Version {
        self.other.conduit_version()
    }
    fn scheme(&self) -> conduit::Scheme {
        self.other.scheme()
    }
    fn host(&self) -> conduit::Host<'_> {
        self.other.host()
    }
    fn virtual_root(&self) -> Option<&str> {
        self.other.virtual_root()
    }
    fn query_string(&self) -> Option<&str> {
        self.other.query_string()
    }
    fn remote_addr(&self) -> SocketAddr {
        self.other.remote_addr()
    }
    fn content_length(&self) -> Option<u64> {
        self.other.content_length()
    }
    fn headers(&self) -> &dyn conduit::Headers {
        self.other.headers()
    }
    fn body(&mut self) -> &mut dyn Read {
        self.other.body()
    }
    fn extensions(&self) -> &conduit::Extensions {
        self.other.extensions()
    }
    fn mut_extensions(&mut self) -> &mut conduit::Extensions {
        self.other.mut_extensions()
    }
}
