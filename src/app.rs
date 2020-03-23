//! Application-wide components in a struct accessible from each request

use crate::{db, Config, Env};
use std::{path::PathBuf, sync::Arc, time::Duration};

use diesel::r2d2;
use oauth2::basic::BasicClient;
use reqwest::blocking::Client;
use scheduled_thread_pool::ScheduledThreadPool;

/// The `App` struct holds the main components of the application like
/// the database connection pool and configurations
// The db, oauth, and git2 types don't implement debug.
#[allow(missing_debug_implementations)]
pub struct App {
    /// The primary database connection pool
    pub primary_database: db::DieselPool,

    /// The read-only replica database connection pool
    pub read_only_replica_database: Option<db::DieselPool>,

    /// The GitHub OAuth2 configuration
    pub github: BasicClient,

    /// A unique key used with conduit_cookie to generate cookies
    pub session_key: String,

    /// The location on disk of the checkout of the crate index git repository
    /// Only used in the development environment.
    pub git_repo_checkout: PathBuf,

    /// The server configuration
    pub config: Config,

    /// A configured client for outgoing HTTP requests
    ///
    /// In production this shares a single connection pool across requests.  In tests
    /// this is either None (in which case any attempt to create an outgoing connection
    /// will panic) or a `Client` configured with a per-test replay proxy.
    pub(crate) http_client: Option<Client>,
}

impl App {
    /// Creates a new `App` with a given `Config` and an optional HTTP `Client`
    ///
    /// Configures and sets up:
    ///
    /// - GitHub OAuth
    /// - Database connection pools
    /// - A `git2::Repository` instance from the index repo checkout (that server.rs ensures exists)
    pub fn new(config: &Config, http_client: Option<Client>) -> App {
        use oauth2::prelude::*;
        use oauth2::{AuthUrl, ClientId, ClientSecret, Scope, TokenUrl};
        use url::Url;

        let github = BasicClient::new(
            ClientId::new(config.gh_client_id.clone()),
            Some(ClientSecret::new(config.gh_client_secret.clone())),
            AuthUrl::new(Url::parse("https://github.com/login/oauth/authorize").unwrap()),
            Some(TokenUrl::new(
                Url::parse("https://github.com/login/oauth/access_token").unwrap(),
            )),
        )
        .add_scope(Scope::new("read:org".to_string()));

        let db_pool_size = match (dotenv::var("DB_POOL_SIZE"), config.env) {
            (Ok(num), _) => num.parse().expect("couldn't parse DB_POOL_SIZE"),
            (_, Env::Production) => 10,
            _ => 1,
        };

        let db_min_idle = match (dotenv::var("DB_MIN_IDLE"), config.env) {
            (Ok(num), _) => Some(num.parse().expect("couldn't parse DB_MIN_IDLE")),
            (_, Env::Production) => Some(5),
            _ => None,
        };

        let db_helper_threads = match (dotenv::var("DB_HELPER_THREADS"), config.env) {
            (Ok(num), _) => num.parse().expect("couldn't parse DB_HELPER_THREADS"),
            (_, Env::Production) => 3,
            _ => 1,
        };

        // Used as the connection and statement timeout value for the database pool(s)
        let db_connection_timeout = match (dotenv::var("DB_TIMEOUT"), config.env) {
            (Ok(num), _) => num.parse().expect("couldn't parse DB_TIMEOUT"),
            (_, Env::Production) => 10,
            (_, Env::Test) => 1,
            _ => 30,
        };

        // Determine if the primary pool is also read-only
        let read_only_mode = dotenv::var("READ_ONLY_MODE").is_ok();
        let primary_db_connection_config = db::ConnectionConfig {
            statement_timeout: db_connection_timeout,
            read_only: read_only_mode,
        };

        let thread_pool = Arc::new(ScheduledThreadPool::new(db_helper_threads));

        let primary_db_config = r2d2::Pool::builder()
            .max_size(db_pool_size)
            .min_idle(db_min_idle)
            .connection_timeout(Duration::from_secs(db_connection_timeout))
            .connection_customizer(Box::new(primary_db_connection_config))
            .thread_pool(thread_pool.clone());

        let primary_database = db::diesel_pool(&config.db_url, config.env, primary_db_config);

        let read_only_replica_database = if let Some(url) = &config.replica_db_url {
            let replica_db_connection_config = db::ConnectionConfig {
                statement_timeout: db_connection_timeout,
                read_only: true,
            };

            let replica_db_config = r2d2::Pool::builder()
                .max_size(db_pool_size)
                .min_idle(db_min_idle)
                .connection_timeout(Duration::from_secs(db_connection_timeout))
                .connection_customizer(Box::new(replica_db_connection_config))
                .thread_pool(thread_pool);

            Some(db::diesel_pool(&url, config.env, replica_db_config))
        } else {
            None
        };

        App {
            primary_database,
            read_only_replica_database,
            github,
            session_key: config.session_key.clone(),
            git_repo_checkout: config.git_repo_checkout.clone(),
            config: config.clone(),
            http_client,
        }
    }

    /// Returns a client for making HTTP requests to upload crate files.
    ///
    /// The client will go through a proxy if the application was configured via
    /// `TestApp::with_proxy()`.
    ///
    /// # Panics
    ///
    /// Panics if the application was not initialized with a client.  This should only occur in
    /// tests that were not properly initialized.
    pub fn http_client(&self) -> &Client {
        self.http_client
            .as_ref()
            .expect("No HTTP client is configured.  In tests, use `TestApp::with_proxy()`.")
    }
}
