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
    builders::PublishBuilder, record, CrateList, CrateResponse, GoodCrate, OkBool, VersionResponse,
};
use cargo_registry::{
    background_jobs::Environment,
    db::DieselPool,
    middleware::current_user::AuthenticationSource,
    models::{ApiToken, User},
    App, Config,
};
use diesel::PgConnection;
use std::{rc::Rc, sync::Arc, time::Duration};
use swirl::Runner;

use conduit::{Handler, Method, Request};
use conduit_test::MockRequest;

use cargo_registry::git::Repository as WorkerRepository;
use git2::Repository as UpstreamRepository;

struct TestAppInner {
    app: Arc<App>,
    // The bomb (if created) needs to be held in scope until the end of the test.
    _bomb: Option<record::Bomb>,
    middle: conduit_middleware::MiddlewareBuilder,
    index: Option<UpstreamRepository>,
    runner: Option<Runner<Environment, DieselPool>>,
}

use swirl::schema::background_jobs;
// FIXME: This is copied from swirl::storage, because it is private
#[derive(Queryable, Identifiable, Debug, Clone)]
struct BackgroundJob {
    pub id: i64,
    pub job_type: String,
    pub data: serde_json::Value,
}

impl Drop for TestAppInner {
    fn drop(&mut self) {
        use diesel::prelude::*;
        use swirl::schema::background_jobs::dsl::*;

        // Avoid a double-panic if the test is already failing
        if std::thread::panicking() {
            return;
        }

        // Lazily run any remaining jobs
        if let Some(runner) = &self.runner {
            runner.run_all_pending_jobs().expect("Could not run jobs");
            runner.assert_no_failed_jobs().expect("Failed jobs remain");
        }

        // Manually verify that all jobs have completed successfully
        // This will catch any tests that enqueued a job but forgot to initialize the runner
        let conn = self.app.diesel_database.get().unwrap();
        let job_count: i64 = background_jobs.count().get_result(&*conn).unwrap();
        assert_eq!(
            0, job_count,
            "Unprocessed or failed jobs remain in the queue"
        );

        // TODO: If a runner was started, obtain the clone from it and ensure its HEAD matches the upstream index HEAD
    }
}

/// A representation of the app and its database transaction
#[derive(Clone)]
pub struct TestApp(Rc<TestAppInner>);

impl TestApp {
    /// Initialize an application with an `Uploader` that panics
    pub fn init() -> TestAppBuilder {
        TestAppBuilder {
            config: crate::simple_config(),
            proxy: None,
            bomb: None,
            index: None,
            build_job_runner: false,
        }
    }

    /// Initialize the app and a proxy that can record and playback outgoing HTTP requests
    pub fn with_proxy() -> TestAppBuilder {
        Self::init().with_proxy()
    }

    /// Initialize a full application, with a proxy, index, and background worker
    pub fn full() -> TestAppBuilder {
        Self::with_proxy().with_git_index().with_job_runner()
    }

    /// Obtain the database connection and pass it to the closure
    ///
    /// Within each test, the connection pool only has 1 connection so it is necessary to drop the
    /// connection before making any API calls.  Once the closure returns, the connection is
    /// dropped, ensuring it is returned to the pool and available for any future API calls.
    pub fn db<T, F: FnOnce(&PgConnection) -> T>(&self, f: F) -> T {
        let conn = self.0.app.diesel_database.get().unwrap();
        f(&conn)
    }

    /// Create a new user with a verified email address in the database and return a mock user
    /// session
    ///
    /// This method updates the database directly
    pub fn db_new_user(&self, username: &str) -> MockCookieUser {
        use cargo_registry::schema::emails;
        use diesel::prelude::*;

        let user = self.db(|conn| {
            let mut user = crate::new_user(username).create_or_update(conn).unwrap();
            let email = "something@example.com";
            user.email = Some(email.to_string());
            diesel::insert_into(emails::table)
                .values((
                    emails::user_id.eq(user.id),
                    emails::email.eq(email),
                    emails::verified.eq(true),
                ))
                .execute(conn)
                .unwrap();
            user
        });
        MockCookieUser {
            app: TestApp(Rc::clone(&self.0)),
            user,
        }
    }

    /// Obtain a reference to the upstream repository ("the index")
    pub fn upstream_repository(&self) -> &UpstreamRepository {
        self.0.index.as_ref().unwrap()
    }

    /// Obtain a list of crates from the index HEAD
    pub fn crates_from_index_head(&self, path: &str) -> Vec<cargo_registry::git::Crate> {
        let path = std::path::Path::new(path);
        let index = self.upstream_repository();
        let tree = index.head().unwrap().peel_to_tree().unwrap();
        let blob = tree
            .get_path(path)
            .unwrap()
            .to_object(&index)
            .unwrap()
            .peel_to_blob()
            .unwrap();
        let content = blob.content();

        // The index format consists of one JSON object per line
        // It is not a JSON array
        let lines = std::str::from_utf8(content).unwrap().lines();
        lines
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    pub fn run_pending_background_jobs(&self) {
        let runner = &self.0.runner;
        let runner = runner.as_ref().expect("Index has not been initialized");

        runner.run_all_pending_jobs().expect("Could not run jobs");
        runner
            .assert_no_failed_jobs()
            .expect("Could not determine if jobs failed");
    }

    /// Obtain a reference to the inner `App` value
    pub fn as_inner(&self) -> &App {
        &*self.0.app
    }
}

pub struct TestAppBuilder {
    config: Config,
    proxy: Option<String>,
    bomb: Option<record::Bomb>,
    index: Option<UpstreamRepository>,
    build_job_runner: bool,
}

impl TestAppBuilder {
    /// Create a `TestApp` with an empty database
    pub fn empty(self) -> (TestApp, MockAnonymousUser) {
        let (app, middle) = crate::build_app(self.config, self.proxy);

        let runner = if self.build_job_runner {
            let connection_pool = app.diesel_database.clone();
            let index =
                WorkerRepository::open(&app.config.index_location).expect("Could not clone index");
            let environment = Environment::new(
                index,
                None,
                connection_pool.clone(),
                app.config.uploader.clone(),
                app.http_client().clone(),
            );

            Some(
                Runner::builder(connection_pool, environment)
                    // We only have 1 connection in tests, so trying to run more than
                    // 1 job concurrently will just block
                    .thread_count(1)
                    .job_start_timeout(Duration::from_secs(1))
                    .build(),
            )
        } else {
            None
        };

        let test_app_inner = TestAppInner {
            app,
            _bomb: self.bomb,
            middle,
            index: self.index,
            runner,
        };
        let test_app = TestApp(Rc::new(test_app_inner));
        let anon = MockAnonymousUser {
            app: test_app.clone(),
        };
        (test_app, anon)
    }

    /// Create a proxy for use with this app
    pub fn with_proxy(mut self) -> Self {
        let (proxy, bomb) = record::proxy();
        self.proxy = Some(proxy);
        self.bomb = Some(bomb);
        self
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

    pub fn with_publish_rate_limit(mut self, rate: Duration, burst: i32) -> Self {
        self.config.publish_rate_limit.rate = rate;
        self.config.publish_rate_limit.burst = burst;
        self
    }

    pub fn with_git_index(mut self) -> Self {
        use crate::git;

        git::init();

        let thread_local_path = git::bare();
        self.index = Some(UpstreamRepository::open_bare(thread_local_path).unwrap());
        self
    }

    pub fn with_job_runner(mut self) -> Self {
        self.build_job_runner = true;
        self
    }
}

/// A colleciton of helper methods for the 3 authentication types
///
/// Helper methods go through public APIs, and should not modify the database directly
pub trait RequestHelper {
    fn request_builder(&self, method: Method, path: &str) -> MockRequest;
    fn app(&self) -> &TestApp;

    /// Run a request
    fn run<T>(&self, mut request: MockRequest) -> Response<T>
    where
        T: serde::de::DeserializeOwned,
    {
        Response::new(self.app().0.middle.call(&mut request))
    }

    /// Issue a GET request
    fn get<T>(&self, path: &str) -> Response<T>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let request = self.request_builder(Method::Get, path);
        self.run(request)
    }

    /// Issue a GET request that includes query parameters
    fn get_with_query<T>(&self, path: &str, query: &str) -> Response<T>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let mut request = self.request_builder(Method::Get, path);
        request.with_query(query);
        self.run(request)
    }

    /// Issue a PUT request
    fn put<T>(&self, path: &str, body: &[u8]) -> Response<T>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let mut request = self.request_builder(Method::Put, path);
        request.with_body(body);
        self.run(request)
    }

    /// Issue a DELETE request
    fn delete<T>(&self, path: &str) -> Response<T>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let request = self.request_builder(Method::Delete, path);
        self.run(request)
    }

    /// Issue a DELETE request with a body... yes we do it, for crate owner removal
    fn delete_with_body<T>(&self, path: &str, body: &[u8]) -> Response<T>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let mut request = self.request_builder(Method::Delete, path);
        request.with_body(body);
        self.run(request)
    }

    /// Search for crates matching a query string
    fn search(&self, query: &str) -> CrateList {
        self.get_with_query("/api/v1/crates", query).good()
    }

    /// Search for crates owned by the specified user.
    fn search_by_user_id(&self, id: i32) -> CrateList {
        self.search(&format!("user_id={}", id))
    }

    /// Enqueue a crate for publishing
    ///
    /// The publish endpoint will enqueue a background job to update the index.  A test must run
    /// any pending background jobs if it intends to observe changes to the index.
    ///
    /// Any pending jobs are run when the `TestApp` is dropped to ensure that the test fails unless
    /// all background tasks complete successfully.
    fn enqueue_publish(&self, publish_builder: PublishBuilder) -> Response<GoodCrate> {
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

    /// Request the JSON used for a crate version's page
    fn show_version(&self, krate_name: &str, version: &str) -> VersionResponse {
        let url = format!("/api/v1/crates/{}/{}", krate_name, version);
        self.get(&url).good()
    }
}

/// A type that can generate unauthenticated requests
pub struct MockAnonymousUser {
    app: TestApp,
}

impl RequestHelper for MockAnonymousUser {
    fn request_builder(&self, method: Method, path: &str) -> MockRequest {
        crate::req(method, path)
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
        let mut request = crate::req(method, path);
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
        let mut request = crate::req(method, path);
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

    /// Add to the specified crate the specified owner.
    pub fn add_named_owner(&self, krate_name: &str, owner: &str) -> Response<OkBool> {
        let url = format!("/api/v1/crates/{}/owners", krate_name);
        let body = format!("{{\"users\":[\"{}\"]}}", owner);
        self.put(&url, body.as_bytes())
    }

    /// Remove from the specified crate the specified owner.
    pub fn remove_named_owner(&self, krate_name: &str, owner: &str) -> Response<OkBool> {
        let url = format!("/api/v1/crates/{}/owners", krate_name);
        let body = format!("{{\"users\":[\"{}\"]}}", owner);
        self.delete_with_body(&url, body.as_bytes())
    }

    /// Add a user as an owner for a crate.
    pub fn add_user_owner(&self, krate_name: &str, user: &User) {
        self.add_named_owner(krate_name, &user.gh_login).good();
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

type ResponseResult = Result<conduit::Response, Box<dyn std::error::Error + Send>>;

/// A type providing helper methods for working with responses
#[must_use]
pub struct Response<T> {
    response: conduit::Response,
    callback_on_good: Option<Box<dyn Fn(&T)>>,
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

    fn with_callback(self, callback_on_good: Box<dyn Fn(&T)>) -> Self {
        Self {
            response: self.response,
            callback_on_good: Some(callback_on_good),
        }
    }

    /// Assert that the response is good and deserialize the message
    pub fn good(mut self) -> T {
        if !crate::ok_resp(&self.response) {
            panic!("bad response: {:?}", self.response.status);
        }
        let good = crate::json(&mut self.response);
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
        match crate::bad_resp(&mut self.response) {
            None => panic!("ok response: {:?}", self.response.status),
            Some(b) => b,
        }
    }

    pub fn assert_status(&self, status: u32) -> &Self {
        assert_eq!(status, self.response.status.0);
        self
    }

    pub fn assert_redirect_ends_with(&self, target: &str) -> &Self {
        assert!(self.response.headers["Location"][0].ends_with(target));
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
