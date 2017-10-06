use std::io::Read;
use std::net::SocketAddr;

use conduit;
use conduit::Request;
use semver;

// Can't derive Debug because of Request.
#[allow(missing_debug_implementations)]
pub struct RequestProxy<'a> {
    pub other: &'a mut (Request + 'a),
    pub path: Option<&'a str>,
    pub method: Option<conduit::Method>,
}

impl<'a> Request for RequestProxy<'a> {
    fn http_version(&self) -> semver::Version {
        self.other.http_version()
    }
    fn conduit_version(&self) -> semver::Version {
        self.other.conduit_version()
    }
    fn method(&self) -> conduit::Method {
        self.method
            .clone()
            .unwrap_or_else(|| self.other.method().clone())
    }
    fn scheme(&self) -> conduit::Scheme {
        self.other.scheme()
    }
    fn host(&self) -> conduit::Host {
        self.other.host()
    }
    fn virtual_root(&self) -> Option<&str> {
        self.other.virtual_root()
    }
    fn path(&self) -> &str {
        self.path.map(|s| &*s).unwrap_or_else(|| self.other.path())
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
    fn headers(&self) -> &conduit::Headers {
        self.other.headers()
    }
    fn body(&mut self) -> &mut Read {
        self.other.body()
    }
    fn extensions(&self) -> &conduit::Extensions {
        self.other.extensions()
    }
    fn mut_extensions(&mut self) -> &mut conduit::Extensions {
        self.other.mut_extensions()
    }
}
