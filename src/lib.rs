extern crate conduit;
extern crate route_recognizer as router;

use std::collections::hash_map::{Entry, HashMap};
use std::error::Error;
use std::fmt;

use conduit::{Handler, Method, Request, Response};
use router::{Match, Router};

pub struct RouteBuilder {
    routers: HashMap<Method, Router<Box<dyn Handler>>>,
}

#[derive(Debug)]
pub struct RouterError(String);

impl RouteBuilder {
    pub fn new() -> RouteBuilder {
        RouteBuilder {
            routers: HashMap::new(),
        }
    }

    pub fn recognize<'a>(
        &'a self,
        method: &Method,
        path: &str,
    ) -> Result<Match<&'a Box<dyn Handler>>, RouterError> {
        match self.routers.get(method) {
            Some(router) => router.recognize(path),
            None => Err(format!("No router found for {:?}", method)),
        }
        .map_err(RouterError)
    }

    pub fn map<'a, H: Handler>(
        &'a mut self,
        method: Method,
        pattern: &str,
        handler: H,
    ) -> &'a mut RouteBuilder {
        {
            let router = match self.routers.entry(method) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => e.insert(Router::new()),
            };
            router.add(pattern, Box::new(handler));
        }
        self
    }

    pub fn get<'a, H: Handler>(&'a mut self, pattern: &str, handler: H) -> &'a mut RouteBuilder {
        self.map(Method::Get, pattern, handler)
    }

    pub fn post<'a, H: Handler>(&'a mut self, pattern: &str, handler: H) -> &'a mut RouteBuilder {
        self.map(Method::Post, pattern, handler)
    }

    pub fn put<'a, H: Handler>(&'a mut self, pattern: &str, handler: H) -> &'a mut RouteBuilder {
        self.map(Method::Put, pattern, handler)
    }

    pub fn delete<'a, H: Handler>(&'a mut self, pattern: &str, handler: H) -> &'a mut RouteBuilder {
        self.map(Method::Delete, pattern, handler)
    }

    pub fn head<'a, H: Handler>(&'a mut self, pattern: &str, handler: H) -> &'a mut RouteBuilder {
        self.map(Method::Head, pattern, handler)
    }
}

impl conduit::Handler for RouteBuilder {
    fn call(&self, request: &mut dyn Request) -> Result<Response, Box<dyn Error + Send>> {
        let m = {
            let method = request.method();
            let path = request.path();

            match self.recognize(&method, path) {
                Ok(m) => m,
                Err(e) => return Err(Box::new(e) as Box<dyn Error + Send>),
            }
        };

        {
            let extensions = request.mut_extensions();
            extensions.insert(m.params.clone());
        }

        (*m.handler).call(request)
    }
}

impl Error for RouterError {
    fn description(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RouterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

pub trait RequestParams<'a> {
    fn params(self) -> &'a router::Params;
}

pub fn params<'a>(req: &'a dyn Request) -> &'a router::Params {
    req.extensions()
        .find::<router::Params>()
        .expect("Missing params")
}

impl<'a> RequestParams<'a> for &'a (dyn Request + 'a) {
    fn params(self) -> &'a router::Params {
        params(self)
    }
}

#[cfg(test)]
mod tests {
    extern crate semver;
    use std::collections::HashMap;
    use std::io;
    use std::net::SocketAddr;

    use {RequestParams, RouteBuilder};

    use conduit;
    use conduit::{Extensions, Handler, Headers, Host, Method, Scheme, TypeMap};

    struct RequestSentinel {
        method: Method,
        path: String,
        extensions: conduit::Extensions,
    }

    impl RequestSentinel {
        fn new(method: Method, path: &'static str) -> RequestSentinel {
            RequestSentinel {
                path: path.to_string(),
                extensions: TypeMap::new(),
                method: method,
            }
        }
    }

    impl conduit::Request for RequestSentinel {
        fn http_version(&self) -> semver::Version {
            unimplemented!()
        }
        fn conduit_version(&self) -> semver::Version {
            unimplemented!()
        }
        fn method(&self) -> Method {
            self.method.clone()
        }
        fn scheme(&self) -> Scheme {
            unimplemented!()
        }
        fn host<'a>(&'a self) -> Host<'a> {
            unimplemented!()
        }
        fn virtual_root<'a>(&'a self) -> Option<&'a str> {
            unimplemented!()
        }
        fn path<'a>(&'a self) -> &'a str {
            &self.path
        }
        fn query_string<'a>(&'a self) -> Option<&'a str> {
            unimplemented!()
        }
        fn remote_addr(&self) -> SocketAddr {
            unimplemented!()
        }
        fn content_length(&self) -> Option<u64> {
            unimplemented!()
        }
        fn headers<'a>(&'a self) -> &'a dyn Headers {
            unimplemented!()
        }
        fn body<'a>(&'a mut self) -> &'a mut dyn io::Read {
            unimplemented!()
        }
        fn extensions<'a>(&'a self) -> &'a Extensions {
            &self.extensions
        }
        fn mut_extensions<'a>(&'a mut self) -> &'a mut Extensions {
            &mut self.extensions
        }
    }

    #[test]
    fn basic_get() {
        let router = test_router();
        let mut req = RequestSentinel::new(Method::Get, "/posts/1");
        let mut res = router.call(&mut req).ok().expect("No response");

        assert_eq!(res.status, (200, "OK"));
        let mut s = Vec::new();
        res.body.write_body(&mut s).unwrap();
        assert_eq!(s, b"1, Get");
    }

    #[test]
    fn basic_post() {
        let router = test_router();
        let mut req = RequestSentinel::new(Method::Post, "/posts/10");
        let mut res = router.call(&mut req).ok().expect("No response");

        assert_eq!(res.status, (200, "OK"));
        let mut s = Vec::new();
        res.body.write_body(&mut s).unwrap();
        assert_eq!(s, b"10, Post");
    }

    #[test]
    fn nonexistent_route() {
        let router = test_router();
        let mut req = RequestSentinel::new(Method::Post, "/nonexistent");
        router.call(&mut req).err().expect("No response");
    }

    fn test_router() -> RouteBuilder {
        let mut router = RouteBuilder::new();
        router.post("/posts/:id", test_handler);
        router.get("/posts/:id", test_handler);
        router
    }

    fn test_handler(req: &mut dyn conduit::Request) -> io::Result<conduit::Response> {
        let mut res = vec![];
        res.push(req.params()["id"].clone());
        res.push(format!("{:?}", req.method()));

        Ok(conduit::Response {
            status: (200, "OK"),
            headers: HashMap::new(),
            body: Box::new(io::Cursor::new(res.join(", ").into_bytes())),
        })
    }
}
