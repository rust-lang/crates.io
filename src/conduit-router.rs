#![feature(macro_rules)]
#![feature(globs)]

extern crate router = "route_recognizer";
extern crate conduit;

use std::collections::HashMap;
use router::{Router, Match};
use conduit::{Method, Handler};

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

#[cfg(test)]
mod tests {
    extern crate semver;
    use std::io::net::ip::IpAddr;
    use std::collections::HashMap;
    use std::io::MemReader;
    use super::*;
    use conduit;
    use conduit::{Method, Scheme, Host, Headers, Extensions};

    struct RequestSentinel;

    impl conduit::Request for RequestSentinel {
        fn http_version(&self) -> semver::Version { unimplemented!() }
        fn conduit_version(&self) -> semver::Version { unimplemented!() }
        fn method(&self) -> Method { unimplemented!() }
        fn scheme(&self) -> Scheme { unimplemented!() }
        fn host<'a>(&'a self) -> Host<'a> { unimplemented!() }
        fn virtual_root<'a>(&'a self) -> Option<&'a str> { unimplemented!() }
        fn path<'a>(&'a self) -> &'a str { unimplemented!() }
        fn query_string<'a>(&'a self) -> Option<&'a str> { unimplemented!() }
        fn remote_ip(&self) -> IpAddr { unimplemented!() }
        fn content_length(&self) -> Option<uint> { unimplemented!() }
        fn headers<'a>(&'a self) -> &'a Headers { unimplemented!() }
        fn body<'a>(&'a mut self) -> &'a mut Reader { unimplemented!() }
        fn extensions<'a>(&'a self) -> &'a Extensions { unimplemented!() }
        fn mut_extensions<'a>(&'a mut self) -> &'a mut Extensions { unimplemented!() }
    }

    #[test]
    fn basic_test() {
        let mut router = RouteBuilder::new();
        router.get("/posts/:id", handler1);

        let m = router.recognize(&conduit::Get, "/posts/1").unwrap();
        let res = (*m.handler).call(&mut RequestSentinel).unwrap();

        assert_eq!(res.status, (200, "OK"));
    }

    fn handler1(_: &mut conduit::Request) -> Result<conduit::Response, ()> {
        Ok(conduit::Response {
            status: (200, "OK"),
            headers: HashMap::new(),
            body: box MemReader::new(vec!())
        })
    }
}
