//! Application-wide components in a struct accessible from each request

use crate::config;
use crate::db::{ConnectionConfig, DieselPool, DieselPooledConn, PoolError};
use std::ops::Deref;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use crate::downloads_counter::DownloadsCounter;
use crate::email::Emails;
use crate::metrics::{InstanceMetrics, ServiceMetrics};
use crate::rate_limiter::RateLimiter;
use crate::storage::Storage;
use axum::extract::{FromRef, FromRequestParts, State};
use crates_io_github::GitHubClient;
use diesel::r2d2;
use moka::future::{Cache, CacheBuilder};
use oauth2::basic::BasicClient;
use scheduled_thread_pool::ScheduledThreadPool;

/// The `App` struct holds the main components of the application like
/// the database connection pool and configurations
pub struct App {
    /// The primary database connection pool
    pub primary_database: DieselPool,

    /// The read-only replica database connection pool
    pub read_only_replica_database: Option<DieselPool>,

    /// GitHub API client
    pub github: Box<dyn GitHubClient>,

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

    /// Storage backend for crate files and other large objects.
    pub storage: Arc<Storage>,

    /// Metrics related to the service as a whole
    pub service_metrics: ServiceMetrics,

    /// Metrics related to this specific instance of the service
    pub instance_metrics: InstanceMetrics,

    /// In-flight request counters for the `balance_capacity` middleware.
    pub balance_capacity: BalanceCapacityState,

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

        let thread_pool = Arc::new(ScheduledThreadPool::new(config.db.helper_threads));

        let primary_database = {
            let primary_db_connection_config = ConnectionConfig {
                statement_timeout: config.db.statement_timeout,
                read_only: config.db.primary.read_only_mode,
            };

            let primary_db_config = r2d2::Pool::builder()
                .max_size(config.db.primary.pool_size)
                .min_idle(config.db.primary.min_idle)
                .connection_timeout(config.db.connection_timeout)
                .connection_customizer(Box::new(primary_db_connection_config))
                .thread_pool(thread_pool.clone());

            DieselPool::new(
                &config.db.primary.url,
                &config.db,
                primary_db_config,
                instance_metrics
                    .database_time_to_obtain_connection
                    .with_label_values(&["primary"]),
            )
            .unwrap()
        };

        let replica_database = if let Some(pool_config) = config.db.replica.as_ref() {
            let replica_db_connection_config = ConnectionConfig {
                statement_timeout: config.db.statement_timeout,
                read_only: true,
            };

            let replica_db_config = r2d2::Pool::builder()
                .max_size(pool_config.pool_size)
                .min_idle(pool_config.min_idle)
                .connection_timeout(config.db.connection_timeout)
                .connection_customizer(Box::new(replica_db_connection_config))
                .thread_pool(thread_pool);

            Some(
                DieselPool::new(
                    &pool_config.url,
                    &config.db,
                    replica_db_config,
                    instance_metrics
                        .database_time_to_obtain_connection
                        .with_label_values(&["follower"]),
                )
                .unwrap(),
            )
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
            version_id_cacher,
            downloads_counter: DownloadsCounter::new(),
            emails,
            storage: Arc::new(Storage::from_config(&config.storage)),
            service_metrics: ServiceMetrics::new().expect("could not initialize service metrics"),
            instance_metrics,
            balance_capacity: Default::default(),
            rate_limiter: RateLimiter::new(config.rate_limiter.clone()),
            config,
        }
    }

    /// A unique key to generate signed cookies
    pub fn session_key(&self) -> &cookie::Key {
        &self.config.session_key
    }

    /// Obtain a read/write database connection from the primary pool
    #[instrument(skip_all)]
    pub fn db_write(&self) -> Result<DieselPooledConn, PoolError> {
        self.primary_database.get()
    }

    /// Obtain a readonly database connection from the replica pool
    ///
    /// If the replica pool is disabled or unavailable, the primary pool is used instead.
    #[instrument(skip_all)]
    pub fn db_read(&self) -> Result<DieselPooledConn, PoolError> {
        let read_only_pool = self.read_only_replica_database.as_ref();
        match read_only_pool.map(|pool| pool.get()) {
            // Replica is available
            Some(Ok(connection)) => Ok(connection),

            // Replica is not available, but primary might be available
            Some(Err(PoolError::UnhealthyPool)) => {
                let _ = self
                    .instance_metrics
                    .database_fallback_used
                    .get_metric_with_label_values(&["follower"])
                    .map(|metric| metric.inc());

                self.primary_database.get()
            }

            // Replica failed
            Some(Err(error)) => Err(error),

            // Replica is disabled, but primary might be available
            None => self.primary_database.get(),
        }
    }

    /// Obtain a readonly database connection from the primary pool
    ///
    /// If the primary pool is unavailable, the replica pool is used instead, if not disabled.
    #[instrument(skip_all)]
    pub fn db_read_prefer_primary(&self) -> Result<DieselPooledConn, PoolError> {
        match (
            self.primary_database.get(),
            &self.read_only_replica_database,
        ) {
            // Primary is available
            (Ok(connection), _) => Ok(connection),

            // Primary is not available, but replica might be available
            (Err(PoolError::UnhealthyPool), Some(read_only_pool)) => {
                let _ = self
                    .instance_metrics
                    .database_fallback_used
                    .get_metric_with_label_values(&["primary"])
                    .map(|metric| metric.inc());

                read_only_pool.get()
            }

            // Primary failed and replica is disabled
            (Err(error), None) => Err(error),

            // Primary failed
            (Err(error), _) => Err(error),
        }
    }
}

#[derive(Debug, Default)]
pub struct BalanceCapacityState {
    pub in_flight_total: AtomicUsize,
    pub in_flight_non_dl_requests: AtomicUsize,
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
