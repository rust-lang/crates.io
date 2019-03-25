use super::response::{Response, ResponseError};
use cargo_registry::models::ApiToken;
use conduit::{Handler, Method};
use conduit_middleware::MiddlewareBuilder;
use conduit_test::MockRequest;

pub struct RequestBuilder<'a> {
    middleware: &'a MiddlewareBuilder,
    request: MockRequest,
}

impl<'a> RequestBuilder<'a> {
    pub(super) fn new(middleware: &'a MiddlewareBuilder, method: Method, path: &str) -> Self {
        Self {
            middleware,
            request: MockRequest::new(method, path),
        }
        .with_header("User-Agent", "conduit-test")
    }

    /// Uses the given token for authentication
    pub fn with_token(self, token: &ApiToken) -> Self {
        self.with_header("Authorization", &token.token)
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.request.header(name, value);
        self
    }

    /// Send the request
    ///
    /// Returns an error if any of the middlewares returned an error, or if
    /// the response status was >= 400.
    pub fn send(mut self) -> Result<Response, ResponseError> {
        Response::new(self.middleware.call(&mut self.request)?)
    }
}
