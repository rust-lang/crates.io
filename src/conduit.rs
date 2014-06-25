extern crate semver;

use std::collections::HashMap;
use std::io::net::ip::IpAddr;

pub trait Request {
    /// The version of HTTP being used
    fn http_version(&self) -> semver::Version;

    /// The version of the conduit spec being used
    fn conduit_version(&self) -> semver::Version;

    /// The request method, such as GET, POST, PUT, DELETE or PATCH
    fn method<'a>(&'a self) -> &'a str;

    /// The scheme part of the request URL
    fn scheme<'a>(&'a self) -> &'a str;

    /// The host part of the requested URL
    fn host<'a>(&'a self) -> &'a str;

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

    /// A Reader for the body of the request
    fn body<'a>(&mut self) -> Box<Reader + Send>;
}

pub struct Response {
    /// The status code as a tuple of the return code and status string
    pub status: (uint, &'static str),

    /// A Map of the headers
    pub headers: HashMap<String, String>,

    /// A Writer for body of the response
    pub body: Box<Reader + Send>
}

pub type Handler = fn(&mut Request) -> Response;
