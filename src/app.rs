//! Application-wide components in a struct accessible from each request

use crate::config;
use crate::db::{connection_url, ConnectionConfig};
use std::ops::Deref;
use std::sync::Arc;

use crate::email::Emails;
use crate::metrics::{InstanceMetrics, ServiceMetrics};
use crate::rate_limiter::RateLimiter;
use crate::storage::Storage;
use axum::extract::{FromRef, FromRequestParts, State};
use crates_io_github::GitHubClient;
use deadpool_diesel::postgres::{Manager as DeadpoolManager, Pool as DeadpoolPool};
use deadpool_diesel::Runtime;
use oauth2::basic::BasicClient;

type DeadpoolResult = Result<deadpool_diesel::postgres::Connection, deadpool_diesel::PoolError>;

/// The `App` struct holds the main components of the application like
/// the database connection pool and configurations
pub struct App {
    /// Database connection pool connected to the primary database
    pub primary_database: DeadpoolPool,

    /// Database connection pool connected to the read-only replica database
    pub replica_database: Option<DeadpoolPool>,

    /// GitHub API client
    pub github: Box<dyn GitHubClient>,

    /// The GitHub OAuth2 configuration
    pub github_oauth: BasicClient,

    /// The server configuration
    pub config: Arc<config::Server>,

    /// Backend used to send emails
    pub emails: Emails,

    /// Storage backend for crate files and other large objects.
    pub storage: Arc<Storage>,

    /// Metrics related to the service as a whole
    pub service_metrics: ServiceMetrics,

    /// Metrics related to this specific instance of the service
    pub instance_metrics: InstanceMetrics,

    /// Rate limit select actions.
    pub rate_limiter: RateLimiter,
}

impl App {
    /// Creates a new `App` with a given `Config` and an optional HTTP `Client`
    ///
    /// Configures and sets up:
    ///
    /// - GitHub OAuth
    /// - Database connection pools
    /// - A `git2::Repository` instance from the index repo checkout (that server.rs ensures exists)
    pub fn new(config: config::Server, emails: Emails, github: Box<dyn GitHubClient>) -> App {
        use oauth2::{AuthUrl, TokenUrl};

        let instance_metrics =
            InstanceMetrics::new().expect("could not initialize instance metrics");

        let github_oauth = BasicClient::new(
            config.gh_client_id.clone(),
            Some(config.gh_client_secret.clone()),
            AuthUrl::new(String::from("https://github.com/login/oauth/authorize")).unwrap(),
            Some(
                TokenUrl::new(String::from("https://github.com/login/oauth/access_token")).unwrap(),
            ),
        );

        let primary_database = {
            use secrecy::ExposeSecret;

            let primary_db_connection_config = ConnectionConfig {
                statement_timeout: config.db.statement_timeout,
                read_only: config.db.primary.read_only_mode,
            };

            let url = connection_url(&config.db, config.db.primary.url.expose_secret());
            let manager = DeadpoolManager::new(url, Runtime::Tokio1);

            DeadpoolPool::builder(manager)
                .runtime(Runtime::Tokio1)
                .max_size(config.db.primary.pool_size)
                .wait_timeout(Some(config.db.connection_timeout))
                .post_create(primary_db_connection_config)
                .build()
                .unwrap()
        };

        let replica_database = if let Some(pool_config) = config.db.replica.as_ref() {
            use secrecy::ExposeSecret;

            let replica_db_connection_config = ConnectionConfig {
                statement_timeout: config.db.statement_timeout,
                read_only: pool_config.read_only_mode,
            };

            let url = connection_url(&config.db, pool_config.url.expose_secret());
            let manager = DeadpoolManager::new(url, Runtime::Tokio1);

            let pool = DeadpoolPool::builder(manager)
                .runtime(Runtime::Tokio1)
                .max_size(pool_config.pool_size)
                .wait_timeout(Some(config.db.connection_timeout))
                .post_create(replica_db_connection_config)
                .build()
                .unwrap();

            Some(pool)
        } else {
            None
        };

        App {
            primary_database,
            replica_database,
            github,
            github_oauth,
            emails,
            storage: Arc::new(Storage::from_config(&config.storage)),
            service_metrics: ServiceMetrics::new().expect("could not initialize service metrics"),
            instance_metrics,
            rate_limiter: RateLimiter::new(config.rate_limiter.clone()),
            config: Arc::new(config),
        }
    }

    /// A unique key to generate signed cookies
    pub fn session_key(&self) -> &cookie::Key {
        &self.config.session_key
    }

    /// Obtain a read/write database connection from the async primary pool
    #[instrument(skip_all)]
    pub async fn db_write(&self) -> DeadpoolResult {
        self.primary_database.get().await
    }

    /// Obtain a readonly database connection from the replica pool
    ///
    /// If the replica pool is disabled or unavailable, the primary pool is used instead.
    #[instrument(skip_all)]
    pub async fn db_read(&self) -> DeadpoolResult {
        let Some(read_only_pool) = self.replica_database.as_ref() else {
            // Replica is disabled, but primary might be available
            return self.primary_database.get().await;
        };

        match read_only_pool.get().await {
            // Replica is available
            Ok(connection) => Ok(connection),

            // Replica is not available, but primary might be available
            Err(deadpool_diesel::PoolError::Backend(error)) => {
                let _ = self
                    .instance_metrics
                    .database_fallback_used
                    .get_metric_with_label_values(&["follower"])
                    .map(|metric| metric.inc());

                warn!("Replica is unavailable, falling back to primary ({error})");
                self.primary_database.get().await
            }

            // Replica failed
            Err(error) => Err(error),
        }
    }

    /// Obtain a readonly database connection from the primary pool
    ///
    /// If the primary pool is unavailable, the replica pool is used instead, if not disabled.
    #[instrument(skip_all)]
    pub async fn db_read_prefer_primary(&self) -> DeadpoolResult {
        let Some(read_only_pool) = self.replica_database.as_ref() else {
            return self.primary_database.get().await;
        };

        match self.primary_database.get().await {
            // Primary is available
            Ok(connection) => Ok(connection),

            // Primary is not available, but replica might be available
            Err(deadpool_diesel::PoolError::Backend(error)) => {
                let _ = self
                    .instance_metrics
                    .database_fallback_used
                    .get_metric_with_label_values(&["primary"])
                    .map(|metric| metric.inc());

                warn!("Primary is unavailable, falling back to replica ({error})");
                read_only_pool.get().await
            }

            // Primary failed
            Err(error) => Err(error),
        }
    }
}

#[derive(Clone, FromRequestParts)]
#[from_request(via(State))]
pub struct AppState(pub Arc<App>);

// deref so you can still access the inner fields easily
impl Deref for AppState {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRef<AppState> for cookie::Key {
    fn from_ref(app: &AppState) -> Self {
        app.session_key().clone()
    }
}
