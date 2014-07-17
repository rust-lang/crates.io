use std::io;
use std::fmt::Show;

use conduit;
use conduit::{Handler, Request, Response};
use conduit_middleware;
use conduit_static::Static;
use semver;

pub struct Middleware {
    handler: Option<Box<Handler + Send + Share>>,
    dist: Static,
}

impl Middleware {
    pub fn new() -> Middleware {
        Middleware {
            handler: None,
            dist: Static::new(Path::new("dist")),
        }
    }
}

impl conduit_middleware::AroundMiddleware for Middleware {
    fn with_handler(&mut self, handler: Box<Handler + Send + Share>) {
        self.handler = Some(handler);
    }
}

impl Handler for Middleware {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Show>> {
        // First, attempt to serve a static file. If we're missing a static
        // file, then keep going.
        match self.dist.call(req) {
            Ok(ref resp) if resp.status.val0() == 404 => {}
            ret => return ret,
        }

        // Second, if we're requesting html, then we've only got one page so
        // serve up that page. Otherwise proxy on to the rest of the app.
        let wants_html = {
            let content = req.headers().find("Accept").unwrap_or(Vec::new());
            content.iter().any(|s| s.contains("html"))
        };
        return if wants_html {
            self.dist.call(&mut RequestProxy {
                other: req,
                path_override: "/index.html",
            })
        } else {
            self.handler.get_ref().call(req)
        };

        struct RequestProxy<'a> {
            other: &'a mut Request,
            path_override: &'a str,
        }
        impl<'a> Request for RequestProxy<'a> {
            fn http_version(&self) -> semver::Version {
                self.other.http_version()
            }
            fn conduit_version(&self) -> semver::Version {
                self.other.conduit_version()
            }
            fn method(&self) -> conduit::Method { self.other.method() }
            fn scheme(&self) -> conduit::Scheme { self.other.scheme() }
            fn host<'a>(&'a self) -> conduit::Host<'a> { self.other.host() }
            fn virtual_root<'a>(&'a self) -> Option<&'a str> {
                self.other.virtual_root()
            }
            fn path<'a>(&'a self) -> &'a str {
                self.path_override.as_slice()
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
    }
}
