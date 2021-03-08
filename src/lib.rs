#![warn(rust_2018_idioms)]

#[macro_use]
extern crate tracing;

use std::collections::hash_map::{Entry, HashMap};
use std::error::Error;
use std::fmt;

use conduit::{box_error, Handler, HandlerResult, Method, RequestExt};
use route_recognizer::{Match, Params, Router};

static UNKNOWN_METHOD: &str = "Invalid method";
static NOT_FOUND: &str = "Path not found";

#[derive(Default)]
pub struct RouteBuilder {
    routers: HashMap<Method, Router<WrappedHandler>>,
}

#[derive(Clone, Copy)]
pub struct RoutePattern(&'static str);

impl RoutePattern {
    pub fn pattern(&self) -> &str {
        self.0
    }
}

pub struct WrappedHandler {
    pattern: RoutePattern,
    handler: Box<dyn Handler>,
}

impl conduit::Handler for WrappedHandler {
    fn call(&self, request: &mut dyn RequestExt) -> HandlerResult {
        self.handler.call(request)
    }
}

#[derive(Debug)]
pub enum RouterError {
    UnknownMethod,
    PathNotFound,
}

impl RouteBuilder {
    pub fn new() -> Self {
        Self {
            routers: HashMap::new(),
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub fn recognize<'a>(
        &'a self,
        method: &Method,
        path: &str,
    ) -> Result<Match<&WrappedHandler>, RouterError> {
        match self.routers.get(method) {
            Some(router) => router.recognize(path).or(Err(RouterError::PathNotFound)),
            None => Err(RouterError::UnknownMethod),
        }
    }

    #[instrument(level = "trace", skip(self, handler))]
    pub fn map<H: Handler>(
        &mut self,
        method: Method,
        pattern: &'static str,
        handler: H,
    ) -> &mut Self {
        {
            let router = match self.routers.entry(method) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => e.insert(Router::new()),
            };
            let wrapped_handler = WrappedHandler {
                pattern: RoutePattern(pattern),
                handler: Box::new(handler),
            };
            router.add(pattern, wrapped_handler);
        }
        self
    }

    pub fn get<H: Handler>(&mut self, pattern: &'static str, handler: H) -> &mut Self {
        self.map(Method::GET, pattern, handler)
    }

    pub fn post<H: Handler>(&mut self, pattern: &'static str, handler: H) -> &mut Self {
        self.map(Method::POST, pattern, handler)
    }

    pub fn put<H: Handler>(&mut self, pattern: &'static str, handler: H) -> &mut Self {
        self.map(Method::PUT, pattern, handler)
    }

    pub fn delete<H: Handler>(&mut self, pattern: &'static str, handler: H) -> &mut Self {
        self.map(Method::DELETE, pattern, handler)
    }

    pub fn head<H: Handler>(&mut self, pattern: &'static str, handler: H) -> &mut Self {
        self.map(Method::HEAD, pattern, handler)
    }
}

impl conduit::Handler for RouteBuilder {
    #[instrument(level = "trace", skip(self, request))]
    fn call(&self, request: &mut dyn RequestExt) -> HandlerResult {
        let mut m = {
            let method = request.method();
            let path = request.path();

            match self.recognize(&method, path) {
                Ok(m) => m,
                Err(e) => {
                    info!("{}", e);
                    return Err(box_error(e));
                }
            }
        };

        // We don't have `pub` access to the fields to destructure `Params`, so swap with an empty
        // value to avoid an allocation.
        let mut params = Params::new();
        std::mem::swap(m.params_mut(), &mut params);

        let pattern = m.handler().pattern;
        debug!(pattern = pattern.0, "matching route handler found");

        {
            let extensions = request.mut_extensions();
            extensions.insert(pattern);
            extensions.insert(params);
        }

        let span = trace_span!("handler", pattern = pattern.0);
        span.in_scope(|| m.handler().call(request))
    }
}

impl Error for RouterError {
    fn description(&self) -> &str {
        match self {
            Self::UnknownMethod => UNKNOWN_METHOD,
            Self::PathNotFound => NOT_FOUND,
        }
    }
}

impl fmt::Display for RouterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[allow(deprecated)]
        self.description().fmt(f)
    }
}

pub trait RequestParams<'a> {
    fn params(self) -> &'a Params;
}

pub fn params(req: &dyn RequestExt) -> &Params {
    req.extensions().find::<Params>().expect("Missing params")
}

impl<'a> RequestParams<'a> for &'a (dyn RequestExt + 'a) {
    fn params(self) -> &'a Params {
        params(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{RequestParams, RouteBuilder, RoutePattern};

    use conduit::{Body, Handler, Method, Response, StatusCode};
    use conduit_test::{MockRequest, ResponseExt};

    lazy_static::lazy_static! {
        static ref TRACING: () = {
            tracing_subscriber::FmtSubscriber::builder()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
                .with_test_writer()
                .init();
        };
    }

    #[test]
    fn basic_get() {
        lazy_static::initialize(&TRACING);

        let router = test_router();
        let mut req = MockRequest::new(Method::GET, "/posts/1");
        let res = router.call(&mut req).expect("No response");

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(*res.into_cow(), b"1, GET, /posts/:id"[..]);
    }

    #[test]
    fn basic_post() {
        lazy_static::initialize(&TRACING);

        let router = test_router();
        let mut req = MockRequest::new(Method::POST, "/posts/10");
        let res = router.call(&mut req).expect("No response");

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(*res.into_cow(), b"10, POST, /posts/:id"[..]);
    }

    #[test]
    fn path_not_found() {
        lazy_static::initialize(&TRACING);

        let router = test_router();
        let mut req = MockRequest::new(Method::POST, "/nonexistent");
        let err = router.call(&mut req).err().unwrap();

        assert_eq!(err.to_string(), super::NOT_FOUND);
    }

    #[test]
    fn unknown_method() {
        lazy_static::initialize(&TRACING);

        let router = test_router();
        let mut req = MockRequest::new(Method::DELETE, "/posts/1");
        let err = router.call(&mut req).err().unwrap();

        assert_eq!(err.to_string(), super::UNKNOWN_METHOD);
    }

    #[test]
    fn catch_all() {
        lazy_static::initialize(&TRACING);

        let mut router = RouteBuilder::new();
        router.get("/*", test_handler);

        let mut req = MockRequest::new(Method::GET, "/foo");
        let res = router.call(&mut req).expect("No response");
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(*res.into_cow(), b", GET, /*"[..]);
    }

    fn test_router() -> RouteBuilder {
        let mut router = RouteBuilder::new();
        router.post("/posts/:id", test_handler);
        router.get("/posts/:id", test_handler);
        router
    }

    fn test_handler(req: &mut dyn conduit::RequestExt) -> conduit::HttpResult {
        let res = vec![
            req.params().find("id").unwrap_or("").to_string(),
            format!("{:?}", req.method()),
            req.extensions()
                .find::<RoutePattern>()
                .unwrap()
                .pattern()
                .to_string(),
        ];

        let bytes = res.join(", ").into_bytes();
        Response::builder().body(Body::from_vec(bytes))
    }
}
