#![feature(core, hash, old_io, alloc)]

extern crate semver;

use std::collections::HashMap;
use std::error::Error;
use std::old_io::net::ip::IpAddr;

pub use self::typemap::TypeMap;
mod typemap;

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Scheme {
    Http,
    Https
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Host<'a> {
    Name(&'a str),
    Ip(IpAddr)
}

#[derive(PartialEq, Hash, Eq, Debug, Clone, Copy)]
pub enum Method {
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
    Other(&'static str)
}

/// A Dictionary for extensions provided by the server or middleware
pub type Extensions = TypeMap;

pub trait Request {
    /// The version of HTTP being used
    fn http_version(&self) -> semver::Version;

    /// The version of the conduit spec being used
    fn conduit_version(&self) -> semver::Version;

    /// The request method, such as GET, POST, PUT, DELETE or PATCH
    fn method(&self) -> Method;

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
    fn content_length(&self) -> Option<u64>;

    /// The request's headers, as conduit::Headers.
    fn headers<'a>(&'a self) -> &'a Headers;

    /// A Reader for the body of the request
    fn body<'a>(&'a mut self) -> &'a mut Reader;

    /// A readable map of extensions
    fn extensions<'a>(&'a self) -> &'a Extensions;

    /// A mutable map of extensions
    fn mut_extensions<'a>(&'a mut self) -> &'a mut Extensions;
}

pub trait Headers {
    /// Find the value of a given header. Multi-line headers are represented
    /// as an array.
    fn find(&self, key: &str) -> Option<Vec<&str>>;

    /// Returns true if a particular header exists
    fn has(&self, key: &str) -> bool;

    /// Iterate over all of the available headers.
    fn all(&self) -> Vec<(&str, Vec<&str>)>;
}

pub struct Response {
    /// The status code as a tuple of the return code and status string
    pub status: (u32, &'static str),

    /// A Map of the headers
    pub headers: HashMap<String, Vec<String>>,

    /// A Writer for body of the response
    pub body: Box<Reader + Send>
}

/// A Handler takes a request and returns a response or an error.
/// By default, a bare function implements `Handler`.
pub trait Handler: Sync + Send + 'static {
    fn call(&self, request: &mut Request) -> Result<Response, Box<Error+Send>>;
}

impl<F, E> Handler for F
    where F: Fn(&mut Request) -> Result<Response, E> + Sync + Send + 'static,
          E: Error + Send + 'static
{
    fn call(&self, request: &mut Request) -> Result<Response, Box<Error+Send>> {
        (*self)(request).map_err(|e| Box::new(e) as Box<Error+Send>)
    }
}
