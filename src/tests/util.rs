//! This module provides utility types and traits for managing a test session
//!
//! Tests start by using one of the `TestApp` constructors: `init`, `with_proxy`, or `full`.  This returns a
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

use crate::{
    builders::PublishBuilder, CategoryListResponse, CategoryResponse, CrateList, CrateResponse,
    GoodCrate, OkBool, OwnersResponse, VersionResponse,
};
use cargo_registry::models::{ApiToken, CreatedApiToken, User};

use conduit::{BoxError, Handler, Method};
use conduit_cookie::SessionMiddleware;
use conduit_test::MockRequest;

use conduit::header;
use cookie::Cookie;
use std::collections::HashMap;

mod chaosproxy;
mod fresh_schema;
mod response;
mod test_app;

pub(crate) use chaosproxy::ChaosProxy;
pub(crate) use fresh_schema::FreshSchema;
pub use response::Response;
pub use test_app::TestApp;

/// This function can be used to create a `Cookie` header for mock requests that
/// include cookie-based authentication.
///
/// ```
/// let cookie = encode_session_header(session_key, user_id);
/// request.header(header::COOKIE, &cookie);
/// ```
///
/// The implementation matches roughly what is happening inside of the
/// `SessionMiddleware` from `conduit_cookie`.
pub fn encode_session_header(session_key: &str, user_id: i32) -> String {
    let cookie_name = "cargo_session";
    let cookie_key = cookie::Key::derive_from(session_key.as_bytes());

    // build session data map
    let mut map = HashMap::new();
    map.insert("user_id".into(), user_id.to_string());

    // encode the map into a cookie value string
    let encoded = SessionMiddleware::encode(&map);

    // put the cookie into a signed cookie jar
    let cookie = Cookie::build(cookie_name, encoded).finish();
    let mut jar = cookie::CookieJar::new();
    jar.signed_mut(&cookie_key).add(cookie);

    // read the raw cookie from the cookie jar
    jar.get(cookie_name).unwrap().to_string()
}

/// A collection of helper methods for the 3 authentication types
///
/// Helper methods go through public APIs, and should not modify the database directly
pub trait RequestHelper {
    fn request_builder(&self, method: Method, path: &str) -> MockRequest;
    fn app(&self) -> &TestApp;

    /// Run a request that is expected to succeed
    #[track_caller]
    fn run<T>(&self, mut request: MockRequest) -> Response<T> {
        Response::new(self.app().as_middleware().call(&mut request))
    }

    /// Run a request that is expected to error
    #[track_caller]
    fn run_err(&self, mut request: MockRequest) -> BoxError {
        self.app().as_middleware().call(&mut request).err().unwrap()
    }

    /// Create a get request
    fn get_request(&self, path: &str) -> MockRequest {
        self.request_builder(Method::GET, path)
    }

    /// Issue a GET request
    #[track_caller]
    fn get<T>(&self, path: &str) -> Response<T> {
        self.run(self.get_request(path))
    }

    /// Issue a GET request that includes query parameters
    #[track_caller]
    fn get_with_query<T>(&self, path: &str, query: &str) -> Response<T> {
        let mut request = self.request_builder(Method::GET, path);
        request.with_query(query);
        self.run(request)
    }

    /// Issue a PUT request
    #[track_caller]
    fn put<T>(&self, path: &str, body: &[u8]) -> Response<T> {
        let mut request = self.request_builder(Method::PUT, path);
        request.with_body(body);
        self.run(request)
    }

    /// Issue a DELETE request
    #[track_caller]
    fn delete<T>(&self, path: &str) -> Response<T> {
        let request = self.request_builder(Method::DELETE, path);
        self.run(request)
    }

    /// Issue a DELETE request with a body... yes we do it, for crate owner removal
    #[track_caller]
    fn delete_with_body<T>(&self, path: &str, body: &[u8]) -> Response<T> {
        let mut request = self.request_builder(Method::DELETE, path);
        request.with_body(body);
        self.run(request)
    }

    /// Search for crates matching a query string
    fn search(&self, query: &str) -> CrateList {
        self.get_with_query("/api/v1/crates", query).good()
    }

    /// Search for crates owned by the specified user.
    fn search_by_user_id(&self, id: i32) -> CrateList {
        self.search(&format!("user_id={id}"))
    }

    /// Enqueue a crate for publishing
    ///
    /// The publish endpoint will enqueue a background job to update the index.  A test must run
    /// any pending background jobs if it intends to observe changes to the index.
    ///
    /// Any pending jobs are run when the `TestApp` is dropped to ensure that the test fails unless
    /// all background tasks complete successfully.
    #[track_caller]
    fn enqueue_publish(&self, publish_builder: PublishBuilder) -> Response<GoodCrate> {
        self.put("/api/v1/crates/new", &publish_builder.body())
    }

    /// Request the JSON used for a crate's page
    fn show_crate(&self, krate_name: &str) -> CrateResponse {
        let url = format!("/api/v1/crates/{krate_name}");
        self.get(&url).good()
    }

    /// Request the JSON used to list a crate's owners
    fn show_crate_owners(&self, krate_name: &str) -> OwnersResponse {
        let url = format!("/api/v1/crates/{krate_name}/owners");
        self.get(&url).good()
    }

    /// Request the JSON used for a crate version's page
    fn show_version(&self, krate_name: &str, version: &str) -> VersionResponse {
        let url = format!("/api/v1/crates/{krate_name}/{version}");
        self.get(&url).good()
    }

    fn show_category(&self, category_name: &str) -> CategoryResponse {
        let url = format!("/api/v1/categories/{category_name}");
        self.get(&url).good()
    }

    fn show_category_list(&self) -> CategoryListResponse {
        let url = "/api/v1/categories";
        self.get(url).good()
    }
}

fn req(method: conduit::Method, path: &str) -> MockRequest {
    let mut request = MockRequest::new(method, path);
    request.header(header::USER_AGENT, "conduit-test");
    request
}

/// A type that can generate unauthenticated requests
pub struct MockAnonymousUser {
    app: TestApp,
}

impl RequestHelper for MockAnonymousUser {
    fn request_builder(&self, method: Method, path: &str) -> MockRequest {
        req(method, path)
    }

    fn app(&self) -> &TestApp {
        &self.app
    }
}

/// A type that can generate cookie authenticated requests
///
/// The `user.id` value is directly injected into a request extension and thus the conduit_cookie
/// session logic is not exercised.
pub struct MockCookieUser {
    app: TestApp,
    user: User,
}

impl RequestHelper for MockCookieUser {
    fn request_builder(&self, method: Method, path: &str) -> MockRequest {
        let session_key = &self.app.as_inner().session_key();
        let cookie = encode_session_header(session_key, self.user.id);

        let mut request = req(method, path);
        request.header(header::COOKIE, &cookie);
        request
    }

    fn app(&self) -> &TestApp {
        &self.app
    }
}

impl MockCookieUser {
    /// Creates an instance from a database `User` instance
    pub fn new(app: &TestApp, user: User) -> Self {
        Self {
            app: app.clone(),
            user,
        }
    }

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
            app: self.app.clone(),
            token,
        }
    }
}

/// A type that can generate token authenticated requests
pub struct MockTokenUser {
    app: TestApp,
    token: CreatedApiToken,
}

impl RequestHelper for MockTokenUser {
    fn request_builder(&self, method: Method, path: &str) -> MockRequest {
        let mut request = req(method, path);
        request.header(header::AUTHORIZATION, &self.token.plaintext);
        request
    }

    fn app(&self) -> &TestApp {
        &self.app
    }
}

impl MockTokenUser {
    /// Returns a reference to the database `ApiToken` model
    pub fn as_model(&self) -> &ApiToken {
        &self.token.model
    }

    pub fn plaintext(&self) -> &str {
        &self.token.plaintext
    }

    /// Add to the specified crate the specified owners.
    pub fn add_named_owners(&self, krate_name: &str, owners: &[&str]) -> Response<OkBool> {
        self.modify_owners(krate_name, owners, Self::put)
    }

    /// Add a single owner to the specified crate.
    pub fn add_named_owner(&self, krate_name: &str, owner: &str) -> Response<OkBool> {
        self.add_named_owners(krate_name, &[owner])
    }

    /// Remove from the specified crate the specified owners.
    pub fn remove_named_owners(&self, krate_name: &str, owners: &[&str]) -> Response<OkBool> {
        self.modify_owners(krate_name, owners, Self::delete_with_body)
    }

    /// Remove a single owner to the specified crate.
    pub fn remove_named_owner(&self, krate_name: &str, owner: &str) -> Response<OkBool> {
        self.remove_named_owners(krate_name, &[owner])
    }

    fn modify_owners<F>(&self, krate_name: &str, owners: &[&str], method: F) -> Response<OkBool>
    where
        F: Fn(&MockTokenUser, &str, &[u8]) -> Response<OkBool>,
    {
        let url = format!("/api/v1/crates/{krate_name}/owners");
        let body = json!({ "owners": owners }).to_string();
        method(self, &url, body.as_bytes())
    }

    /// Add a user as an owner for a crate.
    pub fn add_user_owner(&self, krate_name: &str, username: &str) {
        self.add_named_owner(krate_name, username).good();
    }
}

#[derive(Deserialize, Debug)]
pub struct Error {
    pub detail: String,
}
