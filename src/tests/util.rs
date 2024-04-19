//! This module provides utility types and traits for managing a test session
//!
//! Tests start by using one of the `TestApp` constructors: `init`, `with_proxy`, or `full`.  This returns a
//! `TestAppBuilder` which provides convenience methods for creating up to one user, optionally with
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
    CategoryListResponse, CategoryResponse, CrateList, CrateResponse, GoodCrate, OkBool,
    OwnersResponse, VersionResponse,
};
use crates_io::middleware::session;
use crates_io::models::{ApiToken, CreatedApiToken, User};

use http::{Method, Request};

use axum::body::{Body, Bytes};
use axum::extract::connect_info::MockConnectInfo;
use chrono::NaiveDateTime;
use cookie::Cookie;
use crates_io::models::token::{CrateScope, EndpointScope};
use crates_io::util::token::PlainToken;
use http::header;
use secrecy::ExposeSecret;
use std::collections::HashMap;
use std::net::SocketAddr;
use tower::ServiceExt;

mod chaosproxy;
mod github;
pub mod insta;
pub mod matchers;
mod mock_request;
mod response;
mod test_app;

pub(crate) use chaosproxy::ChaosProxy;
use mock_request::MockRequest;
pub use mock_request::MockRequestExt;
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
/// The implementation matches roughly what is happening inside of our
/// session middleware.
pub fn encode_session_header(session_key: &cookie::Key, user_id: i32) -> String {
    let cookie_name = "cargo_session";

    // build session data map
    let mut map = HashMap::new();
    map.insert("user_id".into(), user_id.to_string());

    // encode the map into a cookie value string
    let encoded = session::encode(&map);

    // put the cookie into a signed cookie jar
    let cookie = Cookie::build((cookie_name, encoded));
    let mut jar = cookie::CookieJar::new();
    jar.signed_mut(session_key).add(cookie);

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
    fn run<T>(&self, request: Request<impl Into<Body>>) -> Response<T> {
        self.app().runtime().block_on(self.async_run(request))
    }

    /// Run a request that is expected to succeed
    async fn async_run<T>(&self, request: Request<impl Into<Body>>) -> Response<T> {
        let app = self.app();
        let router = app.router().clone();

        // Add a mock `SocketAddr` to the requests so that the `ConnectInfo`
        // extractor has something to extract.
        let mocket_addr = SocketAddr::from(([127, 0, 0, 1], 52381));
        let router = router.layer(MockConnectInfo(mocket_addr));

        let request = request.map(Into::into);
        let axum_response = router.oneshot(request).await.unwrap();

        let (parts, body) = axum_response.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        let bytes_response = axum::response::Response::from_parts(parts, bytes);

        Response::new(bytes_response)
    }

    /// Create a get request
    fn get_request(&self, path: &str) -> MockRequest {
        self.request_builder(Method::GET, path)
    }

    /// Create a POST request
    fn post_request(&self, path: &str) -> MockRequest {
        self.request_builder(Method::POST, path)
    }

    /// Issue a GET request
    #[track_caller]
    fn get<T>(&self, path: &str) -> Response<T> {
        self.app().runtime().block_on(self.async_get(path))
    }

    async fn async_get<T>(&self, path: &str) -> Response<T> {
        self.async_run(self.get_request(path)).await
    }

    /// Issue a GET request that includes query parameters
    #[track_caller]
    fn get_with_query<T>(&self, path: &str, query: &str) -> Response<T> {
        self.app()
            .runtime()
            .block_on(self.async_get_with_query(path, query))
    }

    async fn async_get_with_query<T>(&self, path: &str, query: &str) -> Response<T> {
        let path_and_query = format!("{path}?{query}");
        let request = self.request_builder(Method::GET, &path_and_query);
        self.async_run(request).await
    }

    /// Issue a PUT request
    #[track_caller]
    fn put<T>(&self, path: &str, body: impl Into<Bytes>) -> Response<T> {
        self.app().runtime().block_on(self.async_put(path, body))
    }

    async fn async_put<T>(&self, path: &str, body: impl Into<Bytes>) -> Response<T> {
        let body = body.into();
        let is_json = body.starts_with(b"{") && body.ends_with(b"}");

        let mut request = self.request_builder(Method::PUT, path);
        *request.body_mut() = body;
        if is_json {
            request.header(header::CONTENT_TYPE, "application/json");
        }

        self.async_run(request).await
    }

    /// Issue a DELETE request
    #[track_caller]
    fn delete<T>(&self, path: &str) -> Response<T> {
        self.app().runtime().block_on(self.async_delete(path))
    }

    async fn async_delete<T>(&self, path: &str) -> Response<T> {
        let request = self.request_builder(Method::DELETE, path);
        self.async_run(request).await
    }

    /// Issue a DELETE request with a body... yes we do it, for crate owner removal
    #[track_caller]
    fn delete_with_body<T>(&self, path: &str, body: impl Into<Bytes>) -> Response<T> {
        self.app()
            .runtime()
            .block_on(self.async_delete_with_body(path, body))
    }

    async fn async_delete_with_body<T>(&self, path: &str, body: impl Into<Bytes>) -> Response<T> {
        let body = body.into();
        let is_json = body.starts_with(b"{") && body.ends_with(b"}");

        let mut request = self.request_builder(Method::DELETE, path);
        *request.body_mut() = body;
        if is_json {
            request.header(header::CONTENT_TYPE, "application/json");
        }

        self.async_run(request).await
    }

    /// Search for crates matching a query string
    fn search(&self, query: &str) -> CrateList {
        self.app().runtime().block_on(self.async_search(query))
    }

    async fn async_search(&self, query: &str) -> CrateList {
        self.async_get_with_query("/api/v1/crates", query)
            .await
            .good()
    }

    /// Publish the crate and run background jobs to completion
    ///
    /// Background jobs will publish to the git index and sync to the HTTP index.
    #[track_caller]
    fn publish_crate(&self, body: impl Into<Bytes>) -> Response<GoodCrate> {
        self.app()
            .runtime()
            .block_on(self.async_publish_crate(body))
    }

    async fn async_publish_crate(&self, body: impl Into<Bytes>) -> Response<GoodCrate> {
        let response = self.async_put("/api/v1/crates/new", body).await;
        self.app().async_run_pending_background_jobs().await;
        response
    }

    /// Request the JSON used for a crate's page
    fn show_crate(&self, krate_name: &str) -> CrateResponse {
        self.app()
            .runtime()
            .block_on(self.async_show_crate(krate_name))
    }

    async fn async_show_crate(&self, krate_name: &str) -> CrateResponse {
        let url = format!("/api/v1/crates/{krate_name}");
        self.async_get(&url).await.good()
    }

    /// Request the JSON used for a crate's minimal page
    fn show_crate_minimal(&self, krate_name: &str) -> CrateResponse {
        self.app()
            .runtime()
            .block_on(self.async_show_crate_minimal(krate_name))
    }

    async fn async_show_crate_minimal(&self, krate_name: &str) -> CrateResponse {
        let url = format!("/api/v1/crates/{krate_name}");
        self.async_get_with_query(&url, "include=").await.good()
    }

    /// Request the JSON used to list a crate's owners
    fn show_crate_owners(&self, krate_name: &str) -> OwnersResponse {
        self.app()
            .runtime()
            .block_on(self.async_show_crate_owners(krate_name))
    }

    async fn async_show_crate_owners(&self, krate_name: &str) -> OwnersResponse {
        let url = format!("/api/v1/crates/{krate_name}/owners");
        self.async_get(&url).await.good()
    }

    /// Request the JSON used for a crate version's page
    fn show_version(&self, krate_name: &str, version: &str) -> VersionResponse {
        self.app()
            .runtime()
            .block_on(self.async_show_version(krate_name, version))
    }

    async fn async_show_version(&self, krate_name: &str, version: &str) -> VersionResponse {
        let url = format!("/api/v1/crates/{krate_name}/{version}");
        self.async_get(&url).await.good()
    }

    fn show_category(&self, category_name: &str) -> CategoryResponse {
        self.app()
            .runtime()
            .block_on(self.async_show_category(category_name))
    }

    async fn async_show_category(&self, category_name: &str) -> CategoryResponse {
        let url = format!("/api/v1/categories/{category_name}");
        self.async_get(&url).await.good()
    }

    fn show_category_list(&self) -> CategoryListResponse {
        self.app()
            .runtime()
            .block_on(self.async_show_category_list())
    }

    async fn async_show_category_list(&self) -> CategoryListResponse {
        let url = "/api/v1/categories";
        self.async_get(url).await.good()
    }
}

fn req(method: Method, path: &str) -> MockRequest {
    Request::builder()
        .method(method)
        .uri(path)
        .header(header::USER_AGENT, "conduit-test")
        .body(Bytes::new())
        .unwrap()
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
        self.db_new_scoped_token(name, None, None, None)
    }

    /// Creates a scoped token and wraps it in a helper struct
    ///
    /// This method updates the database directly
    pub fn db_new_scoped_token(
        &self,
        name: &str,
        crate_scopes: Option<Vec<CrateScope>>,
        endpoint_scopes: Option<Vec<EndpointScope>>,
        expired_at: Option<NaiveDateTime>,
    ) -> MockTokenUser {
        let token = self.app.db(|conn| {
            ApiToken::insert_with_scopes(
                conn,
                self.user.id,
                name,
                crate_scopes,
                endpoint_scopes,
                expired_at,
            )
            .unwrap()
        });
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
        request.header(header::AUTHORIZATION, self.token.plaintext.expose_secret());
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

    pub fn plaintext(&self) -> &PlainToken {
        &self.token.plaintext
    }

    /// Add to the specified crate the specified owners.
    pub fn add_named_owners(&self, krate_name: &str, owners: &[&str]) -> Response<OkBool> {
        let url = format!("/api/v1/crates/{krate_name}/owners");
        let body = json!({ "owners": owners }).to_string();
        self.put(&url, body)
    }

    /// Add a single owner to the specified crate.
    pub fn add_named_owner(&self, krate_name: &str, owner: &str) -> Response<OkBool> {
        self.add_named_owners(krate_name, &[owner])
    }

    /// Remove from the specified crate the specified owners.
    pub fn remove_named_owners(&self, krate_name: &str, owners: &[&str]) -> Response<OkBool> {
        let url = format!("/api/v1/crates/{krate_name}/owners");
        let body = json!({ "owners": owners }).to_string();
        self.delete_with_body(&url, body)
    }

    /// Remove a single owner to the specified crate.
    pub fn remove_named_owner(&self, krate_name: &str, owner: &str) -> Response<OkBool> {
        self.remove_named_owners(krate_name, &[owner])
    }
}
