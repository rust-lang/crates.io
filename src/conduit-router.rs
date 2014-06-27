#![feature(macro_rules)]
#![feature(globs)]

extern crate router = "route_recognizer";
extern crate conduit;

use std::collections::HashMap;
use std::any::{Any, AnyRefExt};

use router::{Router, Match};
use conduit::{Method, Handler, Request, Response};

pub struct RouteBuilder<E> {
    routers: HashMap<Method, Router<Box<Handler<E>>>>
}

macro_rules! method_map(
    ($method:ident => $variant:ty) => (
        pub fn $method<'a, H: 'static + Handler<E>>(&'a mut self, pattern: &str, handler: H)
                                                -> &'a mut RouteBuilder<E>
        {
            self.map(conduit::$variant, pattern, handler)
        }
    )
)

impl<E> RouteBuilder<E> {
    pub fn new() -> RouteBuilder<E> {
        RouteBuilder { routers: HashMap::new() }
    }

    pub fn recognize<'a>(&'a self, method: &Method, path: &str) -> Result<Match<&'a Box<Handler<E>>>, String> {
        match self.routers.find(method) {
            None => Err(format!("No router found for {}", method)),
            Some(router) => router.recognize(path)
        }
    }

    pub fn map<'a, H: 'static + Handler<E>>(&'a mut self,
                                        method: Method, pattern: &str, handler: H)
                                        -> &'a mut RouteBuilder<E>
    {
        {
            let router = self.routers.find_or_insert_with(method, |_| Router::new());
            router.add(pattern, box handler as Box<Handler<E>>);
        }
        self
    }

    pub fn get<'a, H: 'static + Handler<E>>(&'a mut self, pattern: &str, handler: H)
                                            -> &'a mut RouteBuilder<E>
    {
        self.map(conduit::Get, pattern, handler)
    }

    pub fn post<'a, H: 'static + Handler<E>>(&'a mut self, pattern: &str, handler: H)
                                            -> &'a mut RouteBuilder<E>
    {
        self.map(conduit::Post, pattern, handler)
    }

    pub fn put<'a, H: 'static + Handler<E>>(&'a mut self, pattern: &str, handler: H)
                                            -> &'a mut RouteBuilder<E>
    {
        self.map(conduit::Put, pattern, handler)
    }

    pub fn delete<'a, H: 'static + Handler<E>>(&'a mut self, pattern: &str, handler: H)
                                            -> &'a mut RouteBuilder<E>
    {
        self.map(conduit::Delete, pattern, handler)
    }

    pub fn head<'a, H: 'static + Handler<E>>(&'a mut self, pattern: &str, handler: H)
                                            -> &'a mut RouteBuilder<E>
    {
        self.map(conduit::Head, pattern, handler)
    }
}

impl<E> conduit::Handler<E> for RouteBuilder<E> {
    fn call(&self, request: &mut Request) -> Result<Response, E> {
        let m = {
            let method = request.method();
            let path = request.path();

            self.recognize(&method, path).unwrap()
        };

        {
            let extensions = request.mut_extensions();
            extensions.insert("router.params", box m.params.clone() as Box<Any>);
        }

        (*m.handler).call(request)
    }
}

pub trait RequestParams<'a> {
    fn params(self) -> &'a router::Params;
}

pub fn params<'a>(req: &'a mut Request) -> &'a router::Params {
    req.extensions().find(&"router.params")
        .and_then(|a| a.as_ref::<router::Params>())
        .expect("Missing params")
}

impl<'a> RequestParams<'a> for &'a mut Request {
    fn params(self) -> &'a router::Params {
        params(self)
    }
}

//impl<T: Request> RequestParams for T {}

#[cfg(test)]
mod tests {
    extern crate semver;
    use std::io::net::ip::IpAddr;
    use std::collections::HashMap;
    use std::io::MemReader;
    use super::*;

    use conduit;
    use conduit::{Handler, Method, Scheme, Host, Headers, Extensions};

    struct RequestSentinel {
        method: Method,
        path: String,
        extensions: conduit::Extensions
    }

    impl RequestSentinel {
        fn new(method: Method, path: &'static str) -> RequestSentinel {
            RequestSentinel {
                path: path.to_str(),
                extensions: HashMap::new(),
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
    fn as_conduit_handler() {
        let mut router = RouteBuilder::new();
        router.post("/posts/:id", handler1);
        router.get("/posts/:id", handler1);

        let mut req = RequestSentinel::new(conduit::Get, "/posts/1");
        let mut res = router.call(&mut req).unwrap();

        assert_eq!(res.status, (200, "OK"));
        assert_eq!(res.body.read_to_str().unwrap(), "1, Get".to_str());

        let mut req = RequestSentinel::new(conduit::Post, "/posts/10");
        let mut res = router.call(&mut req).unwrap();

        assert_eq!(res.status, (200, "OK"));
        assert_eq!(res.body.read_to_str().unwrap(), "10, Post".to_str());
    }

    fn handler1(req: &mut conduit::Request) -> Result<conduit::Response, ()> {
        let mut res = vec!();
        res.push(req.params()["id"]);
        res.push(format!("{}", req.method()));

        Ok(conduit::Response {
            status: (200, "OK"),
            headers: HashMap::new(),
            body: box MemReader::new(res.connect(", ").into_bytes())
        })
    }
}
