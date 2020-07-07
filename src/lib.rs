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

    #[allow(clippy::borrowed_box)]
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
    extern crate conduit_test;

    use std::io;
    use std::net::SocketAddr;

    use {RequestParams, RouteBuilder};

    use self::conduit_test::ResponseExt;
    use conduit::{
        Body, Extensions, Handler, HeaderMap, Host, Method, Response, Scheme, StatusCode, TypeMap,
        Version,
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
        fn host(&self) -> Host<'_> {
            unimplemented!()
        }
        fn virtual_root(&self) -> Option<&str> {
            unimplemented!()
        }
        fn path(&self) -> &str {
            &self.path
        }
        fn path_mut(&mut self) -> &mut String {
            &mut self.path
        }
        fn query_string(&self) -> Option<&str> {
            unimplemented!()
        }
        fn remote_addr(&self) -> SocketAddr {
            unimplemented!()
        }
        fn content_length(&self) -> Option<u64> {
            unimplemented!()
        }
        fn headers(&self) -> &HeaderMap {
            unimplemented!()
        }
        fn body(&mut self) -> &mut dyn io::Read {
            unimplemented!()
        }
        fn extensions(&self) -> &Extensions {
            &self.extensions
        }
        fn mut_extensions(&mut self) -> &mut Extensions {
            &mut self.extensions
        }
    }

    #[test]
    fn basic_get() {
        let router = test_router();
        let mut req = RequestSentinel::new(Method::GET, "/posts/1");
        let res = router.call(&mut req).expect("No response");

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(*res.into_cow(), b"1, GET"[..]);
    }

    #[test]
    fn basic_post() {
        let router = test_router();
        let mut req = RequestSentinel::new(Method::POST, "/posts/10");
        let res = router.call(&mut req).expect("No response");

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(*res.into_cow(), b"10, POST"[..]);
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
        Response::builder().body(Body::from_vec(bytes))
    }
}
