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

use crate::models::{ApiToken, CreatedApiToken, User};
use crate::tests::{
    CategoryListResponse, CategoryResponse, CrateList, CrateResponse, GoodCrate, OwnerResp,
    OwnersResponse, VersionResponse,
};

use http::{Method, Request};

use crate::models::token::{CrateScope, EndpointScope};
use crate::util::token::PlainToken;
use axum::body::{Body, Bytes};
use axum::extract::connect_info::MockConnectInfo;
use chrono::NaiveDateTime;
use cookie::Cookie;
use http::header;
use secrecy::ExposeSecret;
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use tower::ServiceExt;

mod chaosproxy;
pub mod github;
pub mod insta;
pub mod matchers;
mod mock_request;
mod response;
mod test_app;

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
    let encoded = crates_io_session::encode(&map);

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
#[allow(async_fn_in_trait)]
pub trait RequestHelper {
    fn request_builder(&self, method: Method, path: &str) -> MockRequest;
    fn app(&self) -> &TestApp;

    /// Run a request that is expected to succeed
    async fn run<T>(&self, request: Request<impl Into<Body>>) -> Response<T> {
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
    async fn get<T>(&self, path: &str) -> Response<T> {
        self.run(self.get_request(path)).await
    }

    /// Issue a GET request that includes query parameters
    async fn get_with_query<T>(&self, path: &str, query: &str) -> Response<T> {
        let path_and_query = format!("{path}?{query}");
        let request = self.request_builder(Method::GET, &path_and_query);
        self.run(request).await
    }

    /// Issue a PUT request
    async fn put<T>(&self, path: &str, body: impl Into<Bytes>) -> Response<T> {
        let body = body.into();

        let mut request = self.request_builder(Method::PUT, path);
        *request.body_mut() = body;
        if is_json_body(request.body()) {
            request.header(header::CONTENT_TYPE, "application/json");
        }

        self.run(request).await
    }

    /// Issue a PATCH request
    async fn patch<T>(&self, path: &str, body: impl Into<Bytes>) -> Response<T> {
        let body = body.into();

        let mut request = self.request_builder(Method::PATCH, path);
        *request.body_mut() = body;
        if is_json_body(request.body()) {
            request.header(header::CONTENT_TYPE, "application/json");
        }

        self.run(request).await
    }

    /// Issue a DELETE request
    async fn delete<T>(&self, path: &str) -> Response<T> {
        let request = self.request_builder(Method::DELETE, path);
        self.run(request).await
    }

    /// Issue a DELETE request with a body... yes we do it, for crate owner removal
    async fn delete_with_body<T>(&self, path: &str, body: impl Into<Bytes>) -> Response<T> {
        let body = body.into();

        let mut request = self.request_builder(Method::DELETE, path);
        *request.body_mut() = body;
        if is_json_body(request.body()) {
            request.header(header::CONTENT_TYPE, "application/json");
        }

        self.run(request).await
    }

    /// Search for crates matching a query string
    async fn search(&self, query: &str) -> CrateList {
        self.get_with_query("/api/v1/crates", query).await.good()
    }

    /// Publish the crate and run background jobs to completion
    ///
    /// Background jobs will publish to the git index and sync to the HTTP index.
    async fn publish_crate(&self, body: impl Into<Bytes>) -> Response<GoodCrate> {
        let response = self.put("/api/v1/crates/new", body).await;
        self.app().run_pending_background_jobs().await;
        response
    }

    /// Request the JSON used for a crate's page
    async fn show_crate(&self, krate_name: &str) -> CrateResponse {
        let url = format!("/api/v1/crates/{krate_name}");
        self.get(&url).await.good()
    }

    /// Request the JSON used to list a crate's owners
    async fn show_crate_owners(&self, krate_name: &str) -> OwnersResponse {
        let url = format!("/api/v1/crates/{krate_name}/owners");
        self.get(&url).await.good()
    }

    /// Request the JSON used for a crate version's page
    async fn show_version(&self, krate_name: &str, version: &str) -> VersionResponse {
        let url = format!("/api/v1/crates/{krate_name}/{version}");
        self.get(&url).await.good()
    }

    async fn show_category(&self, category_name: &str) -> CategoryResponse {
        let url = format!("/api/v1/categories/{category_name}");
        self.get(&url).await.good()
    }

    async fn show_category_list(&self) -> CategoryListResponse {
        let url = "/api/v1/categories";
        self.get(url).await.good()
    }

    /// Add to the specified crate the specified owners.
    async fn add_named_owners<T>(&self, krate_name: &str, owners: &[T]) -> Response<OwnerResp>
    where
        T: serde::Serialize,
    {
        let url = format!("/api/v1/crates/{krate_name}/owners");
        let body = json!({ "owners": owners }).to_string();
        self.put(&url, body).await
    }

    /// Add a single owner to the specified crate.
    async fn add_named_owner(&self, krate_name: &str, owner: &str) -> Response<OwnerResp> {
        self.add_named_owners(krate_name, &[owner]).await
    }

    /// Remove from the specified crate the specified owners.
    async fn remove_named_owners(&self, krate_name: &str, owners: &[&str]) -> Response<OwnerResp> {
        let url = format!("/api/v1/crates/{krate_name}/owners");
        let body = json!({ "owners": owners }).to_string();
        self.delete_with_body(&url, body).await
    }

    /// Remove a single owner to the specified crate.
    async fn remove_named_owner(&self, krate_name: &str, owner: &str) -> Response<OwnerResp> {
        self.remove_named_owners(krate_name, &[owner]).await
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

fn is_json_body(body: &Bytes) -> bool {
    (body.starts_with(b"{") && body.ends_with(b"}"))
        || (body.starts_with(b"[") && body.ends_with(b"]"))
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
    pub async fn db_new_token(&self, name: &str) -> MockTokenUser {
        self.db_new_scoped_token(name, None, None, None).await
    }

    /// Creates a scoped token and wraps it in a helper struct
    ///
    /// This method updates the database directly
    pub async fn db_new_scoped_token(
        &self,
        name: &str,
        crate_scopes: Option<Vec<CrateScope>>,
        endpoint_scopes: Option<Vec<EndpointScope>>,
        expired_at: Option<NaiveDateTime>,
    ) -> MockTokenUser {
        let mut conn = self.app().db_conn().await;

        let token = ApiToken::insert_with_scopes(
            &mut conn,
            self.user.id,
            name,
            crate_scopes,
            endpoint_scopes,
            expired_at,
        )
        .await
        .unwrap();

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
}
