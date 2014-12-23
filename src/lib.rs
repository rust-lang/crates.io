#![feature(macro_rules)]
#![feature(globs)]

extern crate "route-recognizer" as router;
extern crate conduit;

use std::collections::hash_map::{HashMap, Entry};
use std::fmt::Show;

use router::{Router, Match};
use conduit::{Method, Handler, Request, Response};

pub struct RouteBuilder {
    routers: HashMap<Method, Router<Box<Handler + Send + Sync>>>
}

impl RouteBuilder {
    pub fn new() -> RouteBuilder {
        RouteBuilder { routers: HashMap::new() }
    }

    pub fn recognize<'a>(&'a self, method: &Method, path: &str)
                         -> Result<Match<&'a Box<Handler + Send + Sync>>, String>
    {
        match self.routers.get(method) {
            None => Err(format!("No router found for {}", method)),
            Some(router) => router.recognize(path)
        }
    }

    pub fn map<'a, H: Handler>(&'a mut self, method: Method, pattern: &str,
                               handler: H) -> &'a mut RouteBuilder {
        {
            let router = match self.routers.entry(method) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => e.set(Router::new()),
            };
            router.add(pattern, box handler as Box<Handler + Send + Sync>);
        }
        self
    }

    pub fn get<'a, H: Handler>(&'a mut self, pattern: &str, handler: H)
                               -> &'a mut RouteBuilder {
        self.map(Method::Get, pattern, handler)
    }

    pub fn post<'a, H: Handler>(&'a mut self, pattern: &str, handler: H)
                                -> &'a mut RouteBuilder {
        self.map(Method::Post, pattern, handler)
    }

    pub fn put<'a, H: Handler>(&'a mut self, pattern: &str, handler: H)
                               -> &'a mut RouteBuilder {
        self.map(Method::Put, pattern, handler)
    }

    pub fn delete<'a, H: Handler>(&'a mut self, pattern: &str, handler: H)
                                  -> &'a mut RouteBuilder {
        self.map(Method::Delete, pattern, handler)
    }

    pub fn head<'a, H: Handler>(&'a mut self, pattern: &str, handler: H)
                                -> &'a mut RouteBuilder {
        self.map(Method::Head, pattern, handler)
    }
}

impl conduit::Handler for RouteBuilder {
    fn call(&self, request: &mut Request) -> Result<Response, Box<Show + 'static>> {
        let m = {
            let method = request.method();
            let path = request.path();

            match self.recognize(&method, path) {
                Ok(m) => m,
                Err(e) => return Err(box e as Box<Show>)
            }
        };

        {
            let extensions = request.mut_extensions();
            extensions.insert(m.params.clone());
        }

        (*m.handler).call(request)
    }
}

pub trait RequestParams<'a> {
    fn params(self) -> &'a router::Params;
}

pub fn params<'a>(req: &'a Request) -> &'a router::Params {
    req.extensions().find::<router::Params>()
        .expect("Missing params")
}

impl<'a> RequestParams<'a> for &'a (Request + 'a) {
    fn params(self) -> &'a router::Params {
        params(self)
    }
}

#[cfg(test)]
mod tests {
    extern crate semver;
    use std::io::net::ip::IpAddr;
    use std::collections::HashMap;
    use std::io::MemReader;

    use {RouteBuilder, RequestParams};

    use conduit;
    use conduit::{Handler, Method, Scheme, Host, Headers, Extensions, TypeMap};

    struct RequestSentinel {
        method: Method,
        path: String,
        extensions: conduit::Extensions
    }

    impl RequestSentinel {
        fn new(method: Method, path: &'static str) -> RequestSentinel {
            RequestSentinel {
                path: path.to_string(),
                extensions: TypeMap::new(),
                method: method
            }
        }
    }

    impl conduit::Request for RequestSentinel {
        fn http_version(&self) -> semver::Version { unimplemented!() }
        fn conduit_version(&self) -> semver::Version { unimplemented!() }
        fn method(&self) -> Method { self.method }
        fn scheme(&self) -> Scheme { unimplemented!() }
        fn host<'a>(&'a self) -> Host<'a> { unimplemented!() }
        fn virtual_root<'a>(&'a self) -> Option<&'a str> { unimplemented!() }
        fn path<'a>(&'a self) -> &'a str {
            self.path.as_slice()
        }
        fn query_string<'a>(&'a self) -> Option<&'a str> { unimplemented!() }
        fn remote_ip(&self) -> IpAddr { unimplemented!() }
        fn content_length(&self) -> Option<uint> { unimplemented!() }
        fn headers<'a>(&'a self) -> &'a Headers { unimplemented!() }
        fn body<'a>(&'a mut self) -> &'a mut Reader { unimplemented!() }
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
        assert_eq!(res.body.read_to_string().unwrap(), "1, Get".to_string());
    }

    #[test]
    fn basic_post() {
        let router = test_router();
        let mut req = RequestSentinel::new(Method::Post, "/posts/10");
        let mut res = router.call(&mut req).ok().expect("No response");

        assert_eq!(res.status, (200, "OK"));
        assert_eq!(res.body.read_to_string().unwrap(), "10, Post".to_string());
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

    fn test_handler(req: &mut conduit::Request) -> Result<conduit::Response, ()> {
        let mut res = vec!();
        res.push(req.params()["id"].clone());
        res.push(format!("{}", req.method()));

        Ok(conduit::Response {
            status: (200, "OK"),
            headers: HashMap::new(),
            body: box MemReader::new(res.connect(", ").into_bytes())
        })
    }
}
