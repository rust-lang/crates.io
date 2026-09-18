//! Server components accessible from each request.

use crate::config::{DatabasePools, SharedConfig};
use crate::db;
use std::collections::HashMap;
use std::sync::Arc;

use crate::email::Emails;
use crate::metrics::{InstanceMetrics, ServerMetrics, ServiceMetrics};
use crate::rate_limiter::{LimitedAction, RateLimiter, RateLimiterConfig};
use crate::storage::{Storage, StorageConfig};
use axum::extract::{FromRef, FromRequestParts, State};
use bon::Builder;
use crates_io_github::GitHubClient;
use crates_io_trustpub::github::GITHUB_ISSUER_URL;
use crates_io_trustpub::gitlab::GITLAB_ISSUER_URL;
use crates_io_trustpub::keystore::{OidcKeyStore, RealOidcKeyStore};
use derive_more::Deref;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::deadpool::Pool as DeadpoolPool;
use oauth2::basic::BasicClient;
use oauth2::{EndpointNotSet, EndpointSet};
use tracing::{instrument, warn};

type DeadpoolResult = Result<
    diesel_async::pooled_connection::deadpool::Object<AsyncPgConnection>,
    diesel_async::pooled_connection::deadpool::PoolError,
>;

/// Components shared across server requests.
#[doc(hidden)]
#[derive(Builder)]
#[builder(
    builder_type(name = ServerContextBuilder, vis = "pub"),
    state_mod(name = server_context_builder, vis = "pub"),
    finish_fn(name = build_inner, vis = "")
)]
pub struct ServerContextInner {
    /// Database connection pool connected to the primary database
    pub primary_database: DeadpoolPool<AsyncPgConnection>,

    /// Database connection pool connected to the read-only replica database
    pub replica_database: Option<DeadpoolPool<AsyncPgConnection>>,

    /// GitHub API client
    pub github: Arc<dyn GitHubClient>,

    /// The GitHub OAuth2 configuration
    pub github_oauth:
        BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,

    /// OIDC key stores for "Trusted Publishing"
    ///
    /// This is a map of OIDC key stores, where the key is the issuer URL and
    /// the value is the OIDC key store instance.
    #[builder(default)]
    pub oidc_key_stores: HashMap<String, Box<dyn OidcKeyStore>>,

    pub config: Arc<SharedConfig>,

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

    /// OpenTelemetry metrics recorded by the HTTP server
    pub metrics: ServerMetrics,

    /// Rate limit select actions.
    pub rate_limiter: RateLimiter,
}

impl<S: server_context_builder::State> ServerContextBuilder<S> {
    pub fn github_oauth_from_config(
        self,
        config: &SharedConfig,
    ) -> ServerContextBuilder<server_context_builder::SetGithubOauth<S>>
    where
        S::GithubOauth: server_context_builder::IsUnset,
    {
        use oauth2::{AuthUrl, TokenUrl};

        let auth_url = "https://github.com/login/oauth/authorize";
        let auth_url = AuthUrl::new(auth_url.into()).unwrap();
        let token_url = "https://github.com/login/oauth/access_token";
        let token_url = TokenUrl::new(token_url.into()).unwrap();

        let github_oauth = BasicClient::new(config.github_oauth.client_id.clone())
            .set_client_secret(config.github_oauth.client_secret.clone())
            .set_auth_uri(auth_url)
            .set_token_uri(token_url);

        self.github_oauth(github_oauth)
    }

    /// Set the "Trusted Publishing" providers supported by the application.
    ///
    /// This method configures the OIDC key stores for the specified providers
    /// and expects a list of provider names as input.
    ///
    /// Currently, "github" and "gitlab" are supported as providers.
    pub fn trustpub_providers(
        self,
        providers: &[String],
    ) -> ServerContextBuilder<server_context_builder::SetOidcKeyStores<S>>
    where
        S::OidcKeyStores: server_context_builder::IsUnset,
    {
        let mut key_stores: HashMap<String, Box<dyn OidcKeyStore>> = HashMap::new();

        for provider in providers {
            match provider.as_str() {
                "github" => {
                    let key_store = RealOidcKeyStore::new(GITHUB_ISSUER_URL.into());
                    key_stores.insert(GITHUB_ISSUER_URL.into(), Box::new(key_store));
                }
                "gitlab" => {
                    let key_store = RealOidcKeyStore::new(GITLAB_ISSUER_URL.into());
                    key_stores.insert(GITLAB_ISSUER_URL.into(), Box::new(key_store));
                }
                provider => {
                    warn!("Unknown Trusted Publishing provider: {provider}");
                }
            }
        }

        self.oidc_key_stores(key_stores)
    }

    pub fn databases_from_config(
        self,
        config: &DatabasePools,
    ) -> ServerContextBuilder<
        server_context_builder::SetReplicaDatabase<server_context_builder::SetPrimaryDatabase<S>>,
    >
    where
        S::PrimaryDatabase: server_context_builder::IsUnset,
        S::ReplicaDatabase: server_context_builder::IsUnset,
    {
        let primary_database = db::create_pool(&config.primary);
        let replica_database = config.replica.as_ref().map(db::create_pool);

        self.primary_database(primary_database)
            .maybe_replica_database(replica_database)
    }

    pub fn storage_from_config(
        self,
        config: &StorageConfig,
    ) -> ServerContextBuilder<server_context_builder::SetStorage<S>>
    where
        S::Storage: server_context_builder::IsUnset,
    {
        self.storage(Arc::new(Storage::from_config(config)))
    }

    pub fn rate_limiter_from_config(
        self,
        config: HashMap<LimitedAction, RateLimiterConfig>,
    ) -> ServerContextBuilder<server_context_builder::SetRateLimiter<S>>
    where
        S::RateLimiter: server_context_builder::IsUnset,
    {
        self.rate_limiter(RateLimiter::new(config))
    }
}

impl ServerContext {
    /// A unique key to generate signed cookies
    pub fn session_key(&self) -> &cookie::Key {
        &self.config.session_key
    }

    /// Obtains a read/write database connection from the async primary pool
    #[instrument(skip_all)]
    pub async fn db_write(&self) -> DeadpoolResult {
        self.primary_database.get().await
    }

    /// Obtains a readonly database connection from the replica pool
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

                self.metrics.db_fallback("replica");

                warn!("Replica is unavailable, falling back to primary ({error})");
                self.primary_database.get().await
            }
        }
    }

    /// Obtains a readonly database connection from the primary pool
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

                self.metrics.db_fallback("primary");

                warn!("Primary is unavailable, falling back to replica ({error})");
                read_only_pool.get().await
            }
        }
    }
}

/// Components shared across server requests.
#[derive(Clone, FromRequestParts, Deref)]
#[from_request(via(State))]
pub struct ServerContext(Arc<ServerContextInner>);

impl ServerContext {
    /// Creates a builder for a server context.
    pub fn builder() -> ServerContextBuilder {
        ServerContextInner::builder()
    }
}

impl<S: server_context_builder::State> ServerContextBuilder<S> {
    /// Finishes building the server context.
    pub fn build(self) -> ServerContext
    where
        S: server_context_builder::IsComplete,
    {
        ServerContext(Arc::new(self.build_inner()))
    }
}

impl FromRef<ServerContext> for cookie::Key {
    fn from_ref(ctx: &ServerContext) -> Self {
        ctx.session_key().clone()
    }
}
