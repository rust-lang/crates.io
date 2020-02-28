#![warn(rust_2018_idioms)]

extern crate http;

use std::error::Error;
use std::io::{self, prelude::*, Cursor};
use std::net::SocketAddr;

pub use http::{header, HeaderMap, Method, Request, Response, StatusCode, Version};

pub use self::typemap::TypeMap;
mod typemap;

pub type Body = Box<dyn WriteBody>;
pub type ResponseResult<Error> = Result<Response<Body>, Error>;
pub type HttpResult = ResponseResult<http::Error>;

pub type BoxError = Box<dyn Error + Send>;
pub type HandlerResult = Result<Response<Body>, BoxError>;

pub fn static_to_body(bytes: &'static [u8]) -> Body {
    Box::new(bytes)
}

pub fn vec_to_body(bytes: Vec<u8>) -> Body {
    Box::new(Cursor::new(bytes))
}

pub fn box_error<E: Error + Send + 'static>(error: E) -> BoxError {
    Box::new(error)
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Scheme {
    Http,
    Https,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Host<'a> {
    Name(&'a str),
    Socket(SocketAddr),
}

/// A Dictionary for extensions provided by the server or middleware
pub type Extensions = TypeMap;

pub trait RequestExt {
    /// The version of HTTP being used
    fn http_version(&self) -> Version;

    /// The request method, such as GET, POST, PUT, DELETE or PATCH
    fn method(&self) -> &Method;

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
    fn remote_addr(&self) -> SocketAddr;

    /// The byte-size of the body, if any
    fn content_length(&self) -> Option<u64>;

    /// The request's headers, as conduit::Headers.
    fn headers(&self) -> &HeaderMap;

    /// A Reader for the body of the request
    fn body<'a>(&'a mut self) -> &'a mut dyn Read;

    /// A readable map of extensions
    fn extensions<'a>(&'a self) -> &'a Extensions;

    /// A mutable map of extensions
    fn mut_extensions<'a>(&'a mut self) -> &'a mut Extensions;
}

/// A Handler takes a request and returns a response or an error.
/// By default, a bare function implements `Handler`.
pub trait Handler: Sync + Send + 'static {
    fn call(&self, request: &mut dyn RequestExt) -> HandlerResult;
}

impl<F, E> Handler for F
where
    F: Fn(&mut dyn RequestExt) -> ResponseResult<E> + Sync + Send + 'static,
    E: Error + Send + 'static,
{
    fn call(&self, request: &mut dyn RequestExt) -> HandlerResult {
        (*self)(request).map_err(box_error)
    }
}

/// A trait which writes the response body out to a `Write`r.
///
/// This is implemented for all `Read`ers.
pub trait WriteBody {
    fn write_body(&mut self, out: &mut dyn Write) -> io::Result<u64>;
}

impl<R> WriteBody for R
where
    R: Read,
{
    fn write_body(&mut self, out: &mut dyn Write) -> io::Result<u64> {
        io::copy(self, out)
    }
}
