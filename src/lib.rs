#![warn(rust_2018_idioms)]
use conduit::{BoxError, Handler, RequestExt};

pub type BeforeResult = Result<(), BoxError>;
pub type AfterResult = conduit::HandlerResult;

pub trait Middleware: Send + Sync + 'static {
    fn before(&self, _: &mut dyn RequestExt) -> BeforeResult {
        Ok(())
    }

    fn after(&self, _: &mut dyn RequestExt, res: AfterResult) -> AfterResult {
        res
    }
}

pub trait AroundMiddleware: Handler {
    fn with_handler(&mut self, handler: Box<dyn Handler>);
}

pub struct MiddlewareBuilder {
    middlewares: Vec<Box<dyn Middleware>>,
    handler: Option<Box<dyn Handler>>,
}

impl MiddlewareBuilder {
    pub fn new<H: Handler>(handler: H) -> MiddlewareBuilder {
        MiddlewareBuilder {
            middlewares: vec![],
            handler: Some(Box::new(handler) as Box<dyn Handler>),
        }
    }

    pub fn add<M: Middleware>(&mut self, middleware: M) {
        self.middlewares
            .push(Box::new(middleware) as Box<dyn Middleware>);
    }

    pub fn around<M: AroundMiddleware>(&mut self, mut middleware: M) {
        let handler = self.handler.take().unwrap();
        middleware.with_handler(handler);
        self.handler = Some(Box::new(middleware) as Box<dyn Handler>);
    }
}

impl Handler for MiddlewareBuilder {
    fn call(&self, req: &mut dyn RequestExt) -> AfterResult {
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
                let middlewares = &self.middlewares[..i];
                run_afters(middlewares, req, Err(err))
            }
            None => {
                let res = { self.handler.as_ref().unwrap().call(req) };
                let middlewares = &self.middlewares;

                run_afters(middlewares, req, res)
            }
        }
    }
}

fn run_afters(
    middleware: &[Box<dyn Middleware>],
    req: &mut dyn RequestExt,
    res: AfterResult,
) -> AfterResult {
    middleware
        .iter()
        .rev()
        .fold(res, |res, m| m.after(req, res))
}

#[cfg(test)]
mod tests {
    use super::{AfterResult, AroundMiddleware, BeforeResult, Middleware, MiddlewareBuilder};

    use std::any::Any;
    use std::io;
    use std::io::prelude::*;
    use std::net::SocketAddr;

    use conduit_test::ResponseExt;

    use conduit::{
        box_error, Body, Extensions, Handler, HeaderMap, Host, Method, RequestExt, Response,
        Scheme, StatusCode, TypeMap, Version,
    };

    struct RequestSentinel {
        path: String,
        extensions: TypeMap,
        method: Method,
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
        fn body(&mut self) -> &mut dyn Read {
            unimplemented!()
        }
        fn extensions(&self) -> &Extensions {
            &self.extensions
        }
        fn mut_extensions(&mut self) -> &mut Extensions {
            &mut self.extensions
        }
    }

    struct MyMiddleware;

    impl Middleware for MyMiddleware {
        fn before<'a>(&self, req: &'a mut dyn RequestExt) -> BeforeResult {
            req.mut_extensions().insert("hello".to_string());
            Ok(())
        }
    }

    struct ErrorRecovery;

    impl Middleware for ErrorRecovery {
        fn after(&self, _: &mut dyn RequestExt, res: AfterResult) -> AfterResult {
            res.or_else(|e| {
                let e = e.to_string().into_bytes();
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from_vec(e))
                    .map_err(box_error)
            })
        }
    }

    struct ProducesError;

    impl Middleware for ProducesError {
        fn before(&self, _: &mut dyn RequestExt) -> BeforeResult {
            Err(Box::new(io::Error::new(io::ErrorKind::Other, "")))
        }
    }

    struct NotReached;

    impl Middleware for NotReached {
        fn after(&self, _: &mut dyn RequestExt, _: AfterResult) -> AfterResult {
            Response::builder().body(Body::empty()).map_err(box_error)
        }
    }

    struct MyAroundMiddleware {
        handler: Option<Box<dyn Handler>>,
    }

    impl MyAroundMiddleware {
        fn new() -> MyAroundMiddleware {
            MyAroundMiddleware { handler: None }
        }
    }

    impl Middleware for MyAroundMiddleware {}

    impl AroundMiddleware for MyAroundMiddleware {
        fn with_handler(&mut self, handler: Box<dyn Handler>) {
            self.handler = Some(handler)
        }
    }

    impl Handler for MyAroundMiddleware {
        fn call(&self, req: &mut dyn RequestExt) -> AfterResult {
            req.mut_extensions().insert("hello".to_string());
            self.handler.as_ref().unwrap().call(req)
        }
    }

    fn get_extension<T: Any>(req: &dyn RequestExt) -> &T {
        req.extensions().find::<T>().unwrap()
    }

    fn response(string: String) -> Response<Body> {
        Response::builder()
            .body(Body::from_vec(string.into_bytes()))
            .unwrap()
    }

    fn handler(req: &mut dyn RequestExt) -> io::Result<Response<Body>> {
        let hello = get_extension::<String>(req);
        Ok(response(hello.clone()))
    }

    fn error_handler(_: &mut dyn RequestExt) -> io::Result<Response<Body>> {
        Err(io::Error::new(io::ErrorKind::Other, "Error in handler"))
    }

    fn middle_handler(req: &mut dyn RequestExt) -> io::Result<Response<Body>> {
        let hello = get_extension::<String>(req);
        let middle = get_extension::<String>(req);

        Ok(response(format!("{} {}", hello, middle)))
    }

    #[test]
    fn test_simple_middleware() {
        let mut builder = MiddlewareBuilder::new(handler);
        builder.add(MyMiddleware);

        let mut req = RequestSentinel::new(Method::GET, "/");
        let res = builder.call(&mut req).expect("No response");

        assert_eq!(*res.into_cow(), b"hello"[..]);
    }

    #[test]
    fn test_error_recovery() {
        let mut builder = MiddlewareBuilder::new(handler);
        builder.add(ErrorRecovery);
        builder.add(ProducesError);
        // the error bubbles up from ProducesError and shouldn't reach here
        builder.add(NotReached);

        let mut req = RequestSentinel::new(Method::GET, "/");
        let res = builder.call(&mut req).expect("Error not handled");

        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_recovery_in_handlers() {
        let mut builder = MiddlewareBuilder::new(error_handler);
        builder.add(ErrorRecovery);

        let mut req = RequestSentinel::new(Method::GET, "/");
        let res = builder.call(&mut req).expect("Error not handled");

        assert_eq!(*res.into_cow(), b"Error in handler"[..]);
    }

    #[test]
    fn test_around_middleware() {
        let mut builder = MiddlewareBuilder::new(middle_handler);
        builder.add(MyMiddleware);
        builder.around(MyAroundMiddleware::new());

        let mut req = RequestSentinel::new(Method::GET, "/");
        let res = builder.call(&mut req).expect("No response");

        assert_eq!(*res.into_cow(), b"hello hello"[..]);
    }
}
