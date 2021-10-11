//! Application-wide components in a struct accessible from each request

use crate::db::{ConnectionConfig, DieselPool};
use crate::{config, Env};
use std::{sync::Arc, time::Duration};

use crate::downloads_counter::DownloadsCounter;
use crate::email::Emails;
use crate::github::GitHubClient;
use crate::metrics::{InstanceMetrics, ServiceMetrics};
use diesel::r2d2;
use moka::sync::{Cache, CacheBuilder};
use oauth2::basic::BasicClient;
use reqwest::blocking::Client;
use scheduled_thread_pool::ScheduledThreadPool;

/// The `App` struct holds the main components of the application like
/// the database connection pool and configurations
pub struct App {
    /// The primary database connection pool
    pub primary_database: DieselPool,

    /// The read-only replica database connection pool
    pub read_only_replica_database: Option<DieselPool>,

    /// GitHub API client
    pub github: GitHubClient,

    /// The GitHub OAuth2 configuration
    pub github_oauth: BasicClient,

    /// The server configuration
    pub config: config::Server,

    /// Cache the `version_id` of a `canonical_crate_name:semver` pair
    ///
    /// This is used by the download endpoint to reduce the number of database queries. The
    /// `version_id` is only cached under the canonical spelling of the crate name.
    pub(crate) version_id_cacher: Cache<(String, String), i32>,

    /// Count downloads and periodically persist them in the database
    pub downloads_counter: DownloadsCounter,

    /// Backend used to send emails
    pub emails: Emails,

    /// Metrics related to the service as a whole
    pub service_metrics: ServiceMetrics,

    /// Metrics related to this specific instance of the service
    pub instance_metrics: InstanceMetrics,

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
    pub fn new(config: config::Server, http_client: Option<Client>) -> App {
        use oauth2::{AuthUrl, ClientId, ClientSecret, TokenUrl};

        let instance_metrics =
            InstanceMetrics::new().expect("could not initialize instance metrics");

        let github = GitHubClient::new(http_client.clone(), config.gh_base_url.clone());

        let github_oauth = BasicClient::new(
            ClientId::new(config.gh_client_id.clone()),
            Some(ClientSecret::new(config.gh_client_secret.clone())),
            AuthUrl::new(String::from("https://github.com/login/oauth/authorize")).unwrap(),
            Some(
                TokenUrl::new(String::from("https://github.com/login/oauth/access_token")).unwrap(),
            ),
        );

        let db_helper_threads = match (dotenv::var("DB_HELPER_THREADS"), config.env()) {
            (Ok(num), _) => num.parse().expect("couldn't parse DB_HELPER_THREADS"),
            (_, Env::Production) => 3,
            _ => 1,
        };

        // Used as the connection and statement timeout value for the database pool(s)
        let db_connection_timeout = match (dotenv::var("DB_TIMEOUT"), config.env()) {
            (Ok(num), _) => num.parse().expect("couldn't parse DB_TIMEOUT"),
            (_, Env::Production) => 10,
            (_, Env::Test) => 1,
            _ => 30,
        };

        let thread_pool = Arc::new(ScheduledThreadPool::new(db_helper_threads));

        let primary_database = if config.use_test_database_pool {
            DieselPool::new_test(&config.db.primary.url)
        } else {
            let primary_db_connection_config = ConnectionConfig {
                statement_timeout: db_connection_timeout,
                read_only: config.db.primary.read_only_mode,
            };

            let primary_db_config = r2d2::Pool::builder()
                .max_size(config.db.primary.pool_size)
                .min_idle(config.db.primary.min_idle)
                .connection_timeout(Duration::from_secs(db_connection_timeout))
                .connection_customizer(Box::new(primary_db_connection_config))
                .thread_pool(thread_pool.clone());

            DieselPool::new(
                &config.db.primary.url,
                primary_db_config,
                instance_metrics
                    .database_time_to_obtain_connection
                    .with_label_values(&["primary"]),
            )
            .unwrap()
        };

        let replica_database = if let Some(pool_config) = config.db.replica.as_ref() {
            if config.use_test_database_pool {
                Some(DieselPool::new_test(&pool_config.url))
            } else {
                let replica_db_connection_config = ConnectionConfig {
                    statement_timeout: db_connection_timeout,
                    read_only: true,
                };

                let replica_db_config = r2d2::Pool::builder()
                    .max_size(pool_config.pool_size)
                    .min_idle(pool_config.min_idle)
                    .connection_timeout(Duration::from_secs(db_connection_timeout))
                    .connection_customizer(Box::new(replica_db_connection_config))
                    .thread_pool(thread_pool);

                Some(
                    DieselPool::new(
                        &pool_config.url,
                        replica_db_config,
                        instance_metrics
                            .database_time_to_obtain_connection
                            .with_label_values(&["follower"]),
                    )
                    .unwrap(),
                )
            }
        } else {
            None
        };

        let version_id_cacher = CacheBuilder::new(config.version_id_cache_size)
            .time_to_live(config.version_id_cache_ttl)
            .build();

        App {
            primary_database,
            read_only_replica_database: replica_database,
            github,
            github_oauth,
            config,
            version_id_cacher,
            downloads_counter: DownloadsCounter::new(),
            emails: Emails::from_environment(),
            service_metrics: ServiceMetrics::new().expect("could not initialize service metrics"),
            instance_metrics,
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

    /// A unique key used with conduit_cookie to generate signed/encrypted cookies
    pub fn session_key(&self) -> &str {
        &self.config.session_key
    }
}
