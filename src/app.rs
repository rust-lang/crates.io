//! Application-wide components in a struct accessible from each request

use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use diesel::r2d2;
use git2;
use oauth2;
use reqwest;
use scheduled_thread_pool::ScheduledThreadPool;

use crate::util::CargoResult;
use crate::{db, Config, Env};

/// The `App` struct holds the main components of the application like
/// the database connection pool and configurations
// The db, oauth, and git2 types don't implement debug.
#[allow(missing_debug_implementations)]
pub struct App {
    /// The database connection pool
    pub diesel_database: db::DieselPool,

    /// The GitHub OAuth2 configuration
    pub github: oauth2::Config,

    /// A unique key used with conduit_cookie to generate cookies
    pub session_key: String,

    /// The crate index git repository
    pub git_repo: Mutex<git2::Repository>,

    /// The location on disk of the checkout of the crate index git repository
    pub git_repo_checkout: PathBuf,

    /// The server configuration
    pub config: Config,
}

impl App {
    /// Creates a new `App` with a given `Config`
    ///
    /// Configures and sets up:
    ///
    /// - GitHub OAuth
    /// - Database connection pools
    /// - A `git2::Repository` instance from the index repo checkout (that server.rs ensures exists)
    pub fn new(config: &Config) -> App {
        let mut github = oauth2::Config::new(
            &config.gh_client_id,
            &config.gh_client_secret,
            "https://github.com/login/oauth/authorize",
            "https://github.com/login/oauth/access_token",
        );
        github.scopes.push(String::from("read:org"));

        let db_pool_size = match (env::var("DB_POOL_SIZE"), config.env) {
            (Ok(num), _) => num.parse().expect("couldn't parse DB_POOL_SIZE"),
            (_, Env::Production) => 10,
            _ => 1,
        };

        let db_min_idle = match (env::var("DB_MIN_IDLE"), config.env) {
            (Ok(num), _) => Some(num.parse().expect("couldn't parse DB_MIN_IDLE")),
            (_, Env::Production) => Some(5),
            _ => None,
        };

        let db_helper_threads = match (env::var("DB_HELPER_THREADS"), config.env) {
            (Ok(num), _) => num.parse().expect("couldn't parse DB_HELPER_THREADS"),
            (_, Env::Production) => 3,
            _ => 1,
        };

        let db_connection_timeout = match (env::var("DB_TIMEOUT"), config.env) {
            (Ok(num), _) => num.parse().expect("couldn't parse DB_TIMEOUT"),
            (_, Env::Production) => 10,
            _ => 30,
        };

        let thread_pool = Arc::new(ScheduledThreadPool::new(db_helper_threads));

        let diesel_db_config = r2d2::Pool::builder()
            .max_size(db_pool_size)
            .min_idle(db_min_idle)
            .connection_timeout(Duration::from_secs(db_connection_timeout))
            .connection_customizer(Box::new(db::SetStatementTimeout(db_connection_timeout)))
            .thread_pool(thread_pool);

        let repo = git2::Repository::open(&config.git_repo_checkout).unwrap();

        App {
            diesel_database: db::diesel_pool(&config.db_url, diesel_db_config),
            github,
            session_key: config.session_key.clone(),
            git_repo: Mutex::new(repo),
            git_repo_checkout: config.git_repo_checkout.clone(),
            config: config.clone(),
        }
    }

    /// Returns a client for making HTTP requests to upload crate files.
    ///
    /// The handle will go through a proxy if the uploader being used has specified one, which
    /// is only done in tests with `TestApp::with_proxy()` in order to be able to record and
    /// inspect the HTTP requests that tests make.
    pub fn http_client(&self) -> CargoResult<reqwest::Client> {
        let mut builder = reqwest::Client::builder();
        if let Some(proxy) = self.config.uploader.proxy() {
            builder = builder.proxy(reqwest::Proxy::all(proxy)?);
        }
        Ok(builder.build()?)
    }
}
