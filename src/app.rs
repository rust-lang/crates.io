//! Application-wide components in a struct accessible from each request

use crate::config;
use crate::db::{ConnectionConfig, connection_url, make_manager_config};
use std::collections::HashMap;
use std::sync::Arc;

use crate::email::Emails;
use crate::metrics::{InstanceMetrics, ServiceMetrics};
use crate::rate_limiter::{LimitedAction, RateLimiter, RateLimiterConfig};
use crate::storage::{Storage, StorageConfig};
use axum::extract::{FromRef, FromRequestParts, State};
use bon::Builder;
use crates_io_github::GitHubClient;
use deadpool_diesel::Runtime;
use derive_more::Deref;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool as DeadpoolPool;
use oauth2::basic::BasicClient;
use oauth2::{EndpointNotSet, EndpointSet};

type DeadpoolResult = Result<
    diesel_async::pooled_connection::deadpool::Object<AsyncPgConnection>,
    diesel_async::pooled_connection::deadpool::PoolError,
>;

/// The `App` struct holds the main components of the application like
/// the database connection pool and configurations
#[derive(Builder)]
pub struct App {
    /// Database connection pool connected to the primary database
    pub primary_database: DeadpoolPool<AsyncPgConnection>,

    /// Database connection pool connected to the read-only replica database
    pub replica_database: Option<DeadpoolPool<AsyncPgConnection>>,

    /// GitHub API client
    pub github: Box<dyn GitHubClient>,

    /// The GitHub OAuth2 configuration
    pub github_oauth:
        BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,

    /// The server configuration
    pub config: Arc<config::Server>,

    /// Backend used to send emails
    pub emails: Emails,

    /// Storage backend for crate files and other large objects.
    pub storage: Arc<Storage>,

    /// Metrics related to the service as a whole
    #[builder(default = ServiceMetrics::new().expect("could not initialize service metrics"))]
    pub service_metrics: ServiceMetrics,

    /// Metrics related to this specific instance of the service
    #[builder(default = InstanceMetrics::new().expect("could not initialize instance metrics"))]
    pub instance_metrics: InstanceMetrics,

    /// Rate limit select actions.
    pub rate_limiter: RateLimiter,
}

impl<S: app_builder::State> AppBuilder<S> {
    pub fn github_oauth_from_config(
        self,
        config: &config::Server,
    ) -> AppBuilder<app_builder::SetGithubOauth<S>>
    where
        S::GithubOauth: app_builder::IsUnset,
    {
        use oauth2::{AuthUrl, TokenUrl};

        let auth_url = "https://github.com/login/oauth/authorize";
        let auth_url = AuthUrl::new(auth_url.into()).unwrap();
        let token_url = "https://github.com/login/oauth/access_token";
        let token_url = TokenUrl::new(token_url.into()).unwrap();

        let github_oauth = BasicClient::new(config.gh_client_id.clone())
            .set_client_secret(config.gh_client_secret.clone())
            .set_auth_uri(auth_url)
            .set_token_uri(token_url);

        self.github_oauth(github_oauth)
    }

    pub fn databases_from_config(
        self,
        config: &config::DatabasePools,
    ) -> AppBuilder<app_builder::SetReplicaDatabase<app_builder::SetPrimaryDatabase<S>>>
    where
        S::PrimaryDatabase: app_builder::IsUnset,
        S::ReplicaDatabase: app_builder::IsUnset,
    {
        let primary_database = create_database_pool(&config.primary);
        let replica_database = config.replica.as_ref().map(create_database_pool);

        self.primary_database(primary_database)
            .maybe_replica_database(replica_database)
    }

    pub fn storage_from_config(
        self,
        config: &StorageConfig,
    ) -> AppBuilder<app_builder::SetStorage<S>>
    where
        S::Storage: app_builder::IsUnset,
    {
        self.storage(Arc::new(Storage::from_config(config)))
    }

    pub fn rate_limiter_from_config(
        self,
        config: HashMap<LimitedAction, RateLimiterConfig>,
    ) -> AppBuilder<app_builder::SetRateLimiter<S>>
    where
        S::RateLimiter: app_builder::IsUnset,
    {
        self.rate_limiter(RateLimiter::new(config))
    }
}

pub fn create_database_pool(config: &config::DbPoolConfig) -> DeadpoolPool<AsyncPgConnection> {
    let connection_config = ConnectionConfig {
        statement_timeout: config.statement_timeout,
        read_only: config.read_only_mode,
    };

    let url = connection_url(config);
    let manager_config = make_manager_config(config.enforce_tls);
    let manager = AsyncDieselConnectionManager::new_with_config(url, manager_config);

    DeadpoolPool::builder(manager)
        .runtime(Runtime::Tokio1)
        .max_size(config.pool_size)
        .wait_timeout(Some(config.connection_timeout))
        .post_create(connection_config)
        .build()
        .unwrap()
}

impl App {
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
            Err(error) => {
                let _ = self
                    .instance_metrics
                    .database_fallback_used
                    .get_metric_with_label_values(&["follower"])
                    .map(|metric| metric.inc());

                warn!("Replica is unavailable, falling back to primary ({error})");
                self.primary_database.get().await
            }
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
            Err(error) => {
                let _ = self
                    .instance_metrics
                    .database_fallback_used
                    .get_metric_with_label_values(&["primary"])
                    .map(|metric| metric.inc());

                warn!("Primary is unavailable, falling back to replica ({error})");
                read_only_pool.get().await
            }
        }
    }
}

#[derive(Clone, FromRequestParts, Deref)]
#[from_request(via(State))]
pub struct AppState(pub Arc<App>);

impl FromRef<AppState> for cookie::Key {
    fn from_ref(app: &AppState) -> Self {
        app.session_key().clone()
    }
}
