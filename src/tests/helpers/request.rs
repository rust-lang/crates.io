use super::response::{Response, ResponseError};
use cargo_registry::models::{ApiToken, User};
use conduit::{Handler, Method, Request};
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

    /// Sends the request signed in as the given user
    pub fn as_user(mut self, user: &User) -> Self {
        use cargo_registry::middleware::current_user::AuthenticationSource;
        self.request.mut_extensions().insert(user.clone());
        self.request
            .mut_extensions()
            .insert(AuthenticationSource::SessionCookie);
        self
    }

    /// Uses the given token for authentication
    pub fn with_token(self, token: &ApiToken) -> Self {
        self.with_header("Authorization", &token.token)
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.request.header(name, value);
        self
    }

    pub fn with_body<T: Into<Vec<u8>>>(mut self, body: T) -> Self {
        self.request.with_body(&body.into());
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
