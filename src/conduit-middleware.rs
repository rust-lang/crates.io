#![feature(globs)]

extern crate conduit;

use std::fmt::Show;

use conduit::{Request, Response, Handler};

pub trait Middleware {
    #[allow(unused_variable)]
    fn before<'a>(&self,
                  req: &'a mut Request) -> Result<&'a mut Request, Box<Show>> {
        Ok(req)
    }
    #[allow(unused_variable)]
    fn after<'a>(&self, req: &mut Request,
                 res: &'a mut Response) -> Result<&'a mut Response, Box<Show>> {
        Ok(res)
    }
}

pub trait AroundMiddleware : Handler {
    fn with_handler(&mut self, handler: Box<Handler + 'static + Share>);
}

pub struct MiddlewareBuilder {
    middlewares: Vec<Box<Middleware + 'static + Share>>,
    handler: Option<Box<Handler + 'static + Share>>
}

impl MiddlewareBuilder {
    pub fn new<H: Handler + 'static + Share>(handler: H) -> MiddlewareBuilder {
        MiddlewareBuilder {
            middlewares: vec!(),
            handler: Some(box handler as Box<Handler + 'static + Share>)
        }
    }

    pub fn add<M: Middleware + 'static + Share>(&mut self, middleware: M) {
        self.middlewares.push(box middleware as Box<Middleware + 'static + Share>);
    }

    pub fn around<M: AroundMiddleware + 'static + Share>(&mut self,
                                                         mut middleware: M) {
        let handler = self.handler.take_unwrap();
        middleware.with_handler(handler);
        self.handler = Some(box middleware as Box<Handler + 'static + Share>);
    }
}

impl Handler for MiddlewareBuilder {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Show>> {
        for middleware in self.middlewares.iter() {
            try!(middleware.before(req));
        }

        match (self.handler.get_ref()).call(req) {
            Err(err) => return Err(err),
            Ok(mut res) => {
                for middleware in self.middlewares.iter() {
                   try!(middleware.after(req, &mut res));
                }

                Ok(res)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate semver;

    use super::*;

    use std::any::{Any, AnyRefExt};
    use std::io::net::ip::IpAddr;
    use std::io::MemReader;
    use std::fmt::Show;
    use std::collections::HashMap;

    use conduit;
    use conduit::{Request, Response, Host, Headers, Method, Scheme, Extensions, Handler};

    struct RequestSentinel {
        path: String,
        extensions: HashMap<&'static str, Box<Any>>,
        method: Method
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


    struct MyMiddleware;

    impl Middleware for MyMiddleware {
        fn before<'a>(&self, req: &'a mut Request) -> Result<&'a mut Request, Box<Show>> {
            req.mut_extensions().insert("test.middleware", box "hello".to_str() as Box<Any>);
            Ok(req)
        }
    }

    struct MyAroundMiddleware {
        handler: Option<Box<Handler + 'static + Share>>
    }

    impl MyAroundMiddleware {
        fn new() -> MyAroundMiddleware {
            MyAroundMiddleware { handler: None }
        }
    }

    impl Middleware for MyAroundMiddleware {}

    impl AroundMiddleware for MyAroundMiddleware {
        fn with_handler(&mut self, handler: Box<Handler + 'static + Share>) {
            self.handler = Some(handler)
        }
    }

    impl Handler for MyAroundMiddleware {
        fn call(&self, req: &mut Request) -> Result<Response, Box<Show>> {
            req.mut_extensions().insert("test.round-and-round", box "hello".to_str() as Box<Any>);
            self.handler.get_ref().call(req)
        }
    }

    fn get_extension<'a, T: 'static>(req: &'a Request, key: &'static str) -> &'a T {
        req.extensions().find(&key).and_then(|s| s.as_ref::<T>())
            .expect(format!("No {} key found in extensions", key).as_slice())
    }

    fn response(string: String) -> Response {
        Response {
            status: (200, "OK"),
            headers: HashMap::new(),
            body: box MemReader::new(string.into_bytes())
        }
    }

    fn handler(req: &mut Request) -> Result<Response, ()> {
        let hello = get_extension::<String>(req, "test.middleware");
        Ok(response(hello.clone()))
    }

    fn middle_handler(req: &mut Request) -> Result<Response, ()> {
        let hello = get_extension::<String>(req, "test.middleware");
        let middle = get_extension::<String>(req, "test.round-and-round");

        Ok(response(format!("{} {}", hello, middle)))
    }

    #[test]
    fn test_simple_middleware() {

        let mut builder = MiddlewareBuilder::new(handler);
        builder.add(MyMiddleware);

        let mut req = RequestSentinel::new(conduit::Get, "/");
        let mut res = builder.call(&mut req).ok().expect("No response");

        assert_eq!(res.body.read_to_str().ok().expect("No body"), "hello".to_str());
    }

    #[test]
    fn test_around_middleware() {
        let mut builder = MiddlewareBuilder::new(middle_handler);
        builder.add(MyMiddleware);
        builder.around(MyAroundMiddleware::new());

        let mut req = RequestSentinel::new(conduit::Get, "/");
        let mut res = builder.call(&mut req).ok().expect("No response");

        assert_eq!(res.body.read_to_str().ok().expect("No body"), "hello hello".to_str());
    }
}
