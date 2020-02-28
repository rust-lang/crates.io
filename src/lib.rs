#![warn(rust_2018_idioms)]
extern crate conduit;
extern crate route_recognizer as router;

use std::collections::hash_map::{Entry, HashMap};
use std::error::Error;
use std::fmt;

use conduit::{box_error, Handler, HandlerResult, Method, RequestExt};
use router::{Match, Router};

#[derive(Default)]
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
        self.map(Method::GET, pattern, handler)
    }

    pub fn post<'a, H: Handler>(&'a mut self, pattern: &str, handler: H) -> &'a mut RouteBuilder {
        self.map(Method::POST, pattern, handler)
    }

    pub fn put<'a, H: Handler>(&'a mut self, pattern: &str, handler: H) -> &'a mut RouteBuilder {
        self.map(Method::PUT, pattern, handler)
    }

    pub fn delete<'a, H: Handler>(&'a mut self, pattern: &str, handler: H) -> &'a mut RouteBuilder {
        self.map(Method::DELETE, pattern, handler)
    }

    pub fn head<'a, H: Handler>(&'a mut self, pattern: &str, handler: H) -> &'a mut RouteBuilder {
        self.map(Method::HEAD, pattern, handler)
    }
}

impl conduit::Handler for RouteBuilder {
    fn call(&self, request: &mut dyn RequestExt) -> HandlerResult {
        let m = {
            let method = request.method();
            let path = request.path();

            match self.recognize(&method, path) {
                Ok(m) => m,
                Err(e) => return Err(box_error(e)),
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

pub trait RequestParams<'a> {
    fn params(self) -> &'a router::Params;
}

pub fn params(req: &dyn RequestExt) -> &router::Params {
    req.extensions()
        .find::<router::Params>()
        .expect("Missing params")
}

impl<'a> RequestParams<'a> for &'a (dyn RequestExt + 'a) {
    fn params(self) -> &'a router::Params {
        params(self)
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::net::SocketAddr;

    use {RequestParams, RouteBuilder};

    use conduit::{
        vec_to_body, Extensions, Handler, HeaderMap, Host, Method, Response, Scheme, StatusCode,
        TypeMap, Version,
    };

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
                method,
            }
        }
    }

    impl conduit::RequestExt for RequestSentinel {
        fn http_version(&self) -> Version {
            unimplemented!()
        }
        fn method(&self) -> &Method {
            &self.method
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
        fn headers<'a>(&'a self) -> &HeaderMap {
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
        let mut req = RequestSentinel::new(Method::GET, "/posts/1");
        let mut res = router.call(&mut req).ok().expect("No response");

        assert_eq!(res.status(), StatusCode::OK);
        let mut s = Vec::new();
        res.body_mut().write_body(&mut s).unwrap();
        assert_eq!(s, b"1, GET");
    }

    #[test]
    fn basic_post() {
        let router = test_router();
        let mut req = RequestSentinel::new(Method::POST, "/posts/10");
        let mut res = router.call(&mut req).ok().expect("No response");

        assert_eq!(res.status(), StatusCode::OK);
        let mut s = Vec::new();
        res.body_mut().write_body(&mut s).unwrap();
        assert_eq!(s, b"10, POST");
    }

    #[test]
    fn nonexistent_route() {
        let router = test_router();
        let mut req = RequestSentinel::new(Method::POST, "/nonexistent");
        router.call(&mut req).err().expect("No response");
    }

    fn test_router() -> RouteBuilder {
        let mut router = RouteBuilder::new();
        router.post("/posts/:id", test_handler);
        router.get("/posts/:id", test_handler);
        router
    }

    fn test_handler(req: &mut dyn conduit::RequestExt) -> conduit::HttpResult {
        let mut res = vec![];
        res.push(req.params()["id"].clone());
        res.push(format!("{:?}", req.method()));

        let bytes = res.join(", ").into_bytes();
        Response::builder().body(vec_to_body(bytes))
    }
}
