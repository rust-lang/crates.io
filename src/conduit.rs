extern crate semver;

use std::collections::HashMap;
use std::io::net::ip::IpAddr;

#[deriving(PartialEq, Show, Clone)]
pub enum Scheme {
    Http,
    Https
}

#[deriving(PartialEq, Show, Clone)]
pub enum Host<'a> {
    HostName(&'a str),
    HostIp(IpAddr)
}

#[deriving(PartialEq, Show, Clone)]
pub enum Method<'a> {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Connect,
    Options,
    Trace,

    // RFC-5789
    Patch,
    Purge,

    // WebDAV, Subversion, UPNP
    Other(&'a str)
}

pub trait Request {
    /// The version of HTTP being used
    fn http_version(&self) -> semver::Version;

    /// The version of the conduit spec being used
    fn conduit_version(&self) -> semver::Version;

    /// The request method, such as GET, POST, PUT, DELETE or PATCH
    fn method<'a>(&'a self) -> Method<'a>;

    /// The scheme part of the request URL
    fn scheme(&self) -> Scheme;

    /// The host part of the requested URL
    fn host<'a>(&'a self) -> Host<'a>;

    /// The initial part of the request URL's path that corresponds
    /// to a virtual root. This allows an application to have a
    /// virtual location that consumes part of the path.
    fn virtual_root<'a>(&'a self) -> Option<&'a str>;

    /// The remainder of the path.
    fn path<'a>(&'a self) -> &'a str;

    /// The portion of the request URL that follows the "?"
    fn query_string<'a>(&'a self) -> Option<&'a str>;

    /// The remote IP address of the client or the last proxy that
    /// sent the request.
    fn remote_ip(&self) -> IpAddr;

    /// The byte-size of the body, if any
    fn content_length(&self) -> Option<uint>;

    /// The request's headers, as conduit::Headers.
    fn headers<'a>(&'a self) -> &'a Headers;

    /// A Reader for the body of the request
    fn body<'a>(&'a mut self) -> &'a mut Reader;
}

pub type HeaderEntries<'a> = Box<Iterator<(&'a str, Vec<&'a str>)>>;

pub trait Headers {
    /// Find the value of a given header. Multi-line headers are represented
    /// as an array.
    fn find<'a>(&'a self, key: &str) -> Option<Vec<&'a str>>;

    /// Returns true if a particular header exists
    fn has(&self, key: &str) -> bool;

    /// Iterate over all of the available headers.
    fn iter<'a>(&'a self) -> HeaderEntries<'a>;
}

pub struct Response {
    /// The status code as a tuple of the return code and status string
    pub status: (uint, &'static str),

    /// A Map of the headers
    pub headers: HashMap<String, Vec<String>>,

    /// A Writer for body of the response
    pub body: Box<Reader + Send>
}

/// A Handler takes a request and returns a response or an error.
/// By default, a bare function implements `Handler`.
pub trait Handler<E> {
    fn call(&self, request: &mut Request) -> Result<Response, E>;
}

impl<E> Handler<E> for fn(&mut Request) -> Result<Response, E> {
    fn call(&self, request: &mut Request) -> Result<Response, E> {
        (*self)(request)
    }
}
