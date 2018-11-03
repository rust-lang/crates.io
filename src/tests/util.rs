//! This module provides utility types and traits for managing a test session
//!
//! Tests start by using one of the `TestApp` constructors, `init` or `with_proxy`.  This returns a
//! `TestAppBuilder` which provides convience methods for creating up to one user, optionally with
//! a token.  The builder methods all return at least an initialized `TestApp` and a
//! `MockAnonymousUser`.  The `MockAnonymousUser` can be used to issue requests in an
//! unauthenticated session.
//!
//! A `TestApp` value provides raw access to the database through the `db` function and can
//! construct new users via the `db_new_user` function.  This function returns a
//! `MockCookieUser`, which can be used to generate one or more tokens via its `db_new_token`
//! function, which in turn returns a `MockTokenUser`.
//!
//! All three user types implement the `RequestHelper` trait which provides convenience methods for
//! constructing requests.  Some of these methods, such as `publish` are expected to fail for an
//! unauthenticated user (or for other reasons) and return a `Response<T>`.  The `Response<T>`
//! provides several functions to check the response status and deserialize the JSON response.
//!
//! `MockCookieUser` and `MockTokenUser` provide an `as_model` function which returns a reference
//! to the underlying database model value (`User` and `ApiToken` respectively).

use std::{self, rc::Rc, sync::Arc};

use {cargo_registry, conduit, conduit_middleware, diesel, dotenv, serde};

use conduit::{Handler, Method, Request};
use conduit_test::MockRequest;

use builders::PublishBuilder;
use cargo_registry::app::App;
use cargo_registry::middleware::current_user::AuthenticationSource;
use models::{ApiToken, User};

use super::{app, record, CrateList, CrateResponse, GoodCrate};

struct TestAppInner {
    app: Arc<App>,
    // The bomb (if created) needs to be held in scope until the end of the test.
    _bomb: Option<record::Bomb>,
    middle: conduit_middleware::MiddlewareBuilder,
}

/// A representation of the app and its database transaction
pub struct TestApp(Rc<TestAppInner>);

impl TestApp {
    /// Initialize an application with an `Uploader` that panics
    pub fn init() -> TestAppBuilder {
        dotenv::dotenv().ok();
        let (app, middle) = ::simple_app(cargo_registry::Uploader::Panic);
        let inner = Rc::new(TestAppInner {
            app,
            _bomb: None,
            middle,
        });
        TestAppBuilder(TestApp(inner))
    }

    /// Initialize a full application that can record and playback outgoing HTTP requests
    pub fn with_proxy() -> TestAppBuilder {
        let (bomb, app, middle) = app();
        let inner = Rc::new(TestAppInner {
            app,
            _bomb: Some(bomb),
            middle,
        });
        TestAppBuilder(TestApp(inner))
    }

    /// Obtain the database connection and pass it to the closure
    ///
    /// Within each test, the connection pool only has 1 connection so it is necessary to drop the
    /// connection before making any API calls.  Once the closure returns, the connection is
    /// dropped, ensuring it is returned to the pool and available for any future API calls.
    pub fn db<T, F: FnOnce(&DieselConnection) -> T>(&self, f: F) -> T {
        let conn = self.0.app.diesel_database.get().unwrap();
        f(&conn)
    }

    /// Create a new user in the database and return a mock user session
    ///
    /// This method updates the database directly
    pub fn db_new_user(&self, user: &str) -> MockCookieUser {
        let user = self.db(|conn| ::new_user(user).create_or_update(conn).unwrap());
        MockCookieUser {
            app: TestApp(Rc::clone(&self.0)),
            user,
        }
    }

    /// Obtain a reference to the inner `App` value
    pub fn as_inner(&self) -> &App {
        &*self.0.app
    }
}

pub struct TestAppBuilder(TestApp);

impl TestAppBuilder {
    /// Create a `TestApp` with an empty database
    pub fn empty(self) -> (TestApp, MockAnonymousUser) {
        let anon = MockAnonymousUser {
            app: TestApp(Rc::clone(&(self.0).0)),
        };
        (self.0, anon)
    }

    // Create a `TestApp` with a database including a default user
    pub fn with_user(self) -> (TestApp, MockAnonymousUser, MockCookieUser) {
        let (app, anon) = self.empty();
        let user = app.db_new_user("foo");
        (app, anon, user)
    }

    /// Create a `TestApp` with a database including a default user and its token
    pub fn with_token(self) -> (TestApp, MockAnonymousUser, MockCookieUser, MockTokenUser) {
        let (app, anon) = self.empty();
        let user = app.db_new_user("foo");
        let token = user.db_new_token("bar");
        (app, anon, user, token)
    }
}

/// A colleciton of helper methods for the 3 authentication types
///
/// Helper methods go through public APIs, and should not modify the database directly
pub trait RequestHelper {
    fn request_builder(&self, method: Method, path: &str) -> MockRequest;
    fn app(&self) -> &TestApp;

    /// Issue a GET request
    fn get<T>(&self, path: &str) -> Response<T>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let mut request = self.request_builder(Method::Get, path);
        Response::new(self.app().0.middle.call(&mut request))
    }

    /// Issue a GET request that includes query parameters
    fn get_with_query<T>(&self, path: &str, query: &str) -> Response<T>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let mut request = self.request_builder(Method::Get, path);
        request.with_query(query);
        Response::new(self.app().0.middle.call(&mut request))
    }

    /// Issue a PUT request
    fn put<T>(&self, path: &str, body: &[u8]) -> Response<T>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let mut builder = self.request_builder(Method::Put, path);
        let request = builder.with_body(body);
        Response::new(self.app().0.middle.call(request))
    }

    /// Issue a DELETE request
    fn delete<T>(&self, path: &str) -> Response<T>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let mut request = self.request_builder(Method::Delete, path);
        Response::new(self.app().0.middle.call(&mut request))
    }

    /// Search for crates matching a query string
    fn search(&self, query: &str) -> CrateList {
        self.get_with_query("/api/v1/crates", query).good()
    }

    /// Search for crates owned by the specified user.
    fn search_by_user_id(&self, id: i32) -> CrateList {
        self.search(&format!("user_id={}", id))
    }

    /// Publish a crate
    fn publish(&self, publish_builder: PublishBuilder) -> Response<GoodCrate> {
        let krate_name = publish_builder.krate_name.clone();
        let response = self.put("/api/v1/crates/new", &publish_builder.body());
        let callback_on_good = move |json: &GoodCrate| assert_eq!(json.krate.name, krate_name);
        response.with_callback(Box::new(callback_on_good))
    }

    /// Request the JSON used for a crate's page
    fn show_crate(&self, krate_name: &str) -> CrateResponse {
        let url = format!("/api/v1/crates/{}", krate_name);
        self.get(&url).good()
    }
}

/// A type that can generate unauthenticated requests
pub struct MockAnonymousUser {
    app: TestApp,
}

impl RequestHelper for MockAnonymousUser {
    fn request_builder(&self, method: Method, path: &str) -> MockRequest {
        ::req(method, path)
    }

    fn app(&self) -> &TestApp {
        &self.app
    }
}

/// A type that can generate cookie authenticated requests
///
/// The `User` is directly injected into middleware extensions and thus the cookie logic is not
/// exercised.
pub struct MockCookieUser {
    app: TestApp,
    user: User,
}

impl RequestHelper for MockCookieUser {
    fn request_builder(&self, method: Method, path: &str) -> MockRequest {
        let mut request = ::req(method, path);
        request.mut_extensions().insert(self.user.clone());
        request
            .mut_extensions()
            .insert(AuthenticationSource::SessionCookie);
        request
    }

    fn app(&self) -> &TestApp {
        &self.app
    }
}

impl MockCookieUser {
    /// Returns a reference to the database `User` model
    pub fn as_model(&self) -> &User {
        &self.user
    }

    /// Creates a token and wraps it in a helper struct
    ///
    /// This method updates the database directly
    pub fn db_new_token(&self, name: &str) -> MockTokenUser {
        let token = self
            .app
            .db(|conn| ApiToken::insert(conn, self.user.id, name).unwrap());
        MockTokenUser {
            app: TestApp(Rc::clone(&self.app.0)),
            token,
        }
    }
}

/// A type that can generate token authenticated requests
pub struct MockTokenUser {
    app: TestApp,
    token: ApiToken,
}

impl RequestHelper for MockTokenUser {
    fn request_builder(&self, method: Method, path: &str) -> MockRequest {
        let mut request = ::req(method, path);
        request.header("Authorization", &self.token.token);
        request
    }

    fn app(&self) -> &TestApp {
        &self.app
    }
}

impl MockTokenUser {
    /// Returns a reference to the database `ApiToken` model
    pub fn as_model(&self) -> &ApiToken {
        &self.token
    }
}

#[derive(Deserialize, Debug)]
pub struct Error {
    pub detail: String,
}

#[derive(Deserialize)]
pub struct Bad {
    pub errors: Vec<Error>,
}

pub type DieselConnection =
    diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>;
type ResponseResult = Result<conduit::Response, Box<std::error::Error + Send>>;

/// A type providing helper methods for working with responses
#[must_use]
pub struct Response<T> {
    response: conduit::Response,
    callback_on_good: Option<Box<Fn(&T)>>,
}

impl<T> Response<T>
where
    for<'de> T: serde::Deserialize<'de>,
{
    fn new(response: ResponseResult) -> Self {
        Self {
            response: t!(response),
            callback_on_good: None,
        }
    }

    fn with_callback(self, callback_on_good: Box<Fn(&T)>) -> Self {
        Self {
            response: self.response,
            callback_on_good: Some(callback_on_good),
        }
    }

    /// Assert that the response is good and deserialize the message
    pub fn good(mut self) -> T {
        if !::ok_resp(&self.response) {
            panic!("bad response: {:?}", self.response.status);
        }
        let good = ::json(&mut self.response);
        if let Some(callback) = self.callback_on_good {
            callback(&good)
        }
        good
    }

    /// Assert the response status code and deserialze into a list of errors
    ///
    /// Cargo endpoints return a status 200 on error instead of 400.
    pub fn bad_with_status(&mut self, code: u32) -> Bad {
        assert_eq!(self.response.status.0, code);
        match ::bad_resp(&mut self.response) {
            None => panic!("ok response: {:?}", self.response.status),
            Some(b) => b,
        }
    }

    pub fn assert_status(&self, status: u32) -> &Self {
        assert_eq!(status, self.response.status.0);
        self
    }
}

impl Response<()> {
    /// Assert that the status code is 404
    pub fn assert_not_found(&self) {
        assert_eq!((404, "Not Found"), self.response.status);
    }

    /// Assert that the status code is 403
    pub fn assert_forbidden(&self) {
        assert_eq!((403, "Forbidden"), self.response.status);
    }
}
