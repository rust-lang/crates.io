use super::{MockAnonymousUser, MockCookieUser, MockTokenUser};
use crate::record;
use cargo_registry::{
    background_jobs::Environment,
    db::DieselPool,
    git::{Credentials, RepositoryConfig},
    App, Config,
};
use diesel::PgConnection;
use std::{rc::Rc, sync::Arc, time::Duration};
use swirl::Runner;

use cargo_registry::git::Repository as WorkerRepository;
use git2::Repository as UpstreamRepository;

use url::Url;

struct TestAppInner {
    app: Arc<App>,
    // The bomb (if created) needs to be held in scope until the end of the test.
    _bomb: Option<record::Bomb>,
    middle: conduit_middleware::MiddlewareBuilder,
    index: Option<UpstreamRepository>,
    runner: Option<Runner<Environment, DieselPool>>,
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
            runner.check_for_failed_jobs().expect("Failed jobs remain");
        }

        // Manually verify that all jobs have completed successfully
        // This will catch any tests that enqueued a job but forgot to initialize the runner
        let conn = self.app.primary_database.get().unwrap();
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
        init_logger();

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
        let conn = self.0.app.primary_database.get().unwrap();
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
            let email = "something@example.com";

            let user = crate::new_user(username)
                .create_or_update(None, conn)
                .unwrap();
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
            app: self.clone(),
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
            .check_for_failed_jobs()
            .expect("Could not determine if jobs failed");
    }

    /// Obtain a reference to the inner `App` value
    pub fn as_inner(&self) -> &App {
        &self.0.app
    }

    /// Obtain a reference to the inner middleware builder
    pub fn as_middleware(&self) -> &conduit_middleware::MiddlewareBuilder {
        &self.0.middle
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
        use crate::git;

        let (app, middle) = crate::build_app(self.config, self.proxy);

        let runner = if self.build_job_runner {
            let repository_config = RepositoryConfig {
                index_location: Url::from_file_path(&git::bare()).unwrap(),
                credentials: Credentials::Missing,
            };
            let index = WorkerRepository::open(&repository_config).expect("Could not clone index");
            let environment = Environment::new(
                index,
                app.config.uploader.clone(),
                app.http_client().clone(),
            );

            Some(
                Runner::builder(environment)
                    // We only have 1 connection in tests, so trying to run more than
                    // 1 job concurrently will just block
                    .thread_count(1)
                    .connection_pool(app.primary_database.clone())
                    .job_start_timeout(Duration::from_secs(5))
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

    pub fn with_config(mut self, f: impl FnOnce(&mut Config)) -> Self {
        f(&mut self.config);
        self
    }

    pub fn with_publish_rate_limit(self, rate: Duration, burst: i32) -> Self {
        self.with_config(|config| {
            config.publish_rate_limit.rate = rate;
            config.publish_rate_limit.burst = burst;
        })
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

pub fn init_logger() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .with_test_writer()
        .try_init();
}
