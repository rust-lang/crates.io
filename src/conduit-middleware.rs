#![feature(globs)]

extern crate conduit;

use std::fmt::Show;

use conduit::{Request, Response, Handler};

pub trait Middleware {
    fn before(&self, _: &mut Request) -> Result<(), Box<Show>> {
        Ok(())
    }

    fn after(&self, _: &mut Request, res: Result<Response, Box<Show>>)
             -> Result<Response, Box<Show>>
    {
        res
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
        let mut error = None;

        for (i, middleware) in self.middlewares.iter().enumerate() {
            match middleware.before(req) {
                Ok(_) => (),
                Err(err) => {
                    error = Some((err, i));
                    break;
                }
            }
        }

        match error {
            Some((err, i)) => {
                let middlewares = self.middlewares.slice_to(i);
                run_afters(middlewares, req, Err(err))
            },
            None => {
                let res = { self.handler.get_ref().call(req) };
                let middlewares = self.middlewares.as_slice();

                run_afters(middlewares, req, res)
            }
        }
    }
}

fn run_afters(middleware: &[Box<Middleware>],
                  req: &mut Request,
                  res: Result<Response, Box<Show>>)
                  -> Result<Response, Box<Show>>
{
    middleware.iter().rev().fold(res, |res, m| m.after(req, res))
}

#[cfg(test)]
mod tests {
    extern crate semver;

    use super::*;

    use std::any::{Any, AnyRefExt};
    use std::io::net::ip::IpAddr;
    use std::io::MemReader;
    use std::fmt;
    use std::fmt::{Show, Formatter};
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
        fn before<'a>(&self, req: &'a mut Request) -> Result<(), Box<Show>> {
            req.mut_extensions().insert("test.middleware", box "hello".to_str() as Box<Any>);
            Ok(())
        }
    }

    struct ErrorRecovery;

    impl Middleware for ErrorRecovery {
        fn after(&self, _: &mut Request, res: Result<Response, Box<Show>>)
                     -> Result<Response, Box<Show>>
        {
            res.or_else(|e| {
                Ok(Response {
                    status: (500, "Internal Server Error"),
                    headers: HashMap::new(),
                    body: box MemReader::new(show(e).to_str().into_bytes())
                })
            })
        }
    }

    struct ProducesError;

    impl Middleware for ProducesError {
        fn before(&self, _: &mut Request) -> Result<(), Box<Show>> {
            Err(box "Nope".to_str() as Box<Show>)
        }
    }

    struct NotReached;

    impl Middleware for NotReached {
        fn after(&self, _: &mut Request, _: Result<Response, Box<Show>>)
                     -> Result<Response, Box<Show>>
        {
            Ok(Response {
                status: (200, "OK"),
                headers: HashMap::new(),
                body: box MemReader::new(vec!())
            })
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

    struct Shower<'a> {
        inner: &'a Show
    }

    impl<'a> Show for Shower<'a> {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            self.inner.fmt(f)
        }
    }

    fn show<'a>(s: &'a Show) -> Shower<'a> {
        Shower { inner: s }
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

    fn error_handler(_: &mut Request) -> Result<Response, String> {
        Err("Error in handler".to_str())
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
    fn test_error_recovery() {
        let mut builder = MiddlewareBuilder::new(handler);
        builder.add(ErrorRecovery);
        builder.add(ProducesError);
        // the error bubbles up from ProducesError and shouldn't reach here
        builder.add(NotReached);

        let mut req = RequestSentinel::new(conduit::Get, "/");
        let res = builder.call(&mut req).ok().expect("Error not handled");

        assert_eq!(res.status, (500, "Internal Server Error"));
    }

    #[test]
    fn test_error_recovery_in_handlers() {
        let mut builder = MiddlewareBuilder::new(error_handler);
        builder.add(ErrorRecovery);

        let mut req = RequestSentinel::new(conduit::Get, "/");
        let mut res = builder.call(&mut req).ok().expect("Error not handled");

        assert_eq!(res.status, (500, "Internal Server Error"));
        assert_eq!(res.body.read_to_str().ok().expect("No body"), "Error in handler".to_str());
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
