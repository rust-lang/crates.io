use std::io;

use conduit;
use conduit::Request;
use semver;

pub struct RequestProxy<'a> {
    pub other: &'a mut Request + 'a,
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
        self.method.unwrap_or(self.other.method())
    }
    fn scheme(&self) -> conduit::Scheme { self.other.scheme() }
    fn host<'a>(&'a self) -> conduit::Host<'a> { self.other.host() }
    fn virtual_root<'a>(&'a self) -> Option<&'a str> {
        self.other.virtual_root()
    }
    fn path<'b>(&'b self) -> &'b str {
        self.path.map(|s| &*s).unwrap_or(self.other.path())
    }
    fn query_string<'a>(&'a self) -> Option<&'a str> {
        self.other.query_string()
    }
    fn remote_ip(&self) -> io::net::ip::IpAddr { self.other.remote_ip() }
    fn content_length(&self) -> Option<uint> {
        self.other.content_length()
    }
    fn headers<'a>(&'a self) -> &'a conduit::Headers {
        self.other.headers()
    }
    fn body<'a>(&'a mut self) -> &'a mut Reader { self.other.body() }
    fn extensions<'a>(&'a self) -> &'a conduit::Extensions {
        self.other.extensions()
    }
    fn mut_extensions<'a>(&'a mut self) -> &'a mut conduit::Extensions {
        self.other.mut_extensions()
    }
}
