use deadpool_diesel::postgres::{Hook, HookError};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, CustomizeConnection, State};
use prometheus::Histogram;
use secrecy::{ExposeSecret, SecretString};
use std::ops::Deref;
use std::time::Duration;
use thiserror::Error;
use url::Url;

use crate::config;

pub mod sql_types;

pub type ConnectionPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct DieselPool {
    pool: ConnectionPool,
    time_to_obtain_connection_metric: Option<Histogram>,
}

impl DieselPool {
    pub(crate) fn new(
        url: &SecretString,
        config: &config::DatabasePools,
        r2d2_config: r2d2::Builder<ConnectionManager<PgConnection>>,
        time_to_obtain_connection_metric: Histogram,
    ) -> Result<DieselPool, PoolError> {
        let manager = ConnectionManager::new(connection_url(config, url.expose_secret()));

        // For crates.io we want the behavior of creating a database pool to be slightly different
        // than the defaults of R2D2: the library's build() method assumes its consumers always
        // need a database connection to operate, so it blocks creating a pool until a minimum
        // number of connections is available.
        //
        // crates.io can actually operate in a limited capacity without a database connections,
        // especially by serving download requests to our users. Because of that we don't want to
        // block indefinitely waiting for a connection: we instead need to wait for a bit (to avoid
        // serving errors for the first connections until the pool is initialized) and if we can't
        // establish any connection continue booting up the application. The database pool will
        // automatically be marked as unhealthy and the rest of the application will adapt.
        let pool = DieselPool {
            pool: r2d2_config.build_unchecked(manager),
            time_to_obtain_connection_metric: Some(time_to_obtain_connection_metric),
        };
        match pool.wait_until_healthy(Duration::from_secs(5)) {
            Ok(()) => {}
            Err(PoolError::UnhealthyPool) => {}
            Err(err) => return Err(err),
        }

        Ok(pool)
    }

    pub fn new_background_worker(pool: r2d2::Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            pool,
            time_to_obtain_connection_metric: None,
        }
    }

    #[instrument(name = "db.connect", skip_all)]
    pub fn get(&self) -> Result<DieselPooledConn, PoolError> {
        match self.time_to_obtain_connection_metric.as_ref() {
            Some(time_to_obtain_connection_metric) => time_to_obtain_connection_metric
                .observe_closure_duration(|| {
                    if let Some(conn) = self.pool.try_get() {
                        Ok(conn)
                    } else if !self.is_healthy() {
                        Err(PoolError::UnhealthyPool)
                    } else {
                        Ok(self.pool.get()?)
                    }
                }),
            None => Ok(self.pool.get()?),
        }
    }

    pub fn state(&self) -> State {
        self.pool.state()
    }

    #[instrument(skip_all)]
    pub fn wait_until_healthy(&self, timeout: Duration) -> Result<(), PoolError> {
        match self.pool.get_timeout(timeout) {
            Ok(_) => Ok(()),
            Err(_) if !self.is_healthy() => Err(PoolError::UnhealthyPool),
            Err(err) => Err(PoolError::R2D2(err)),
        }
    }

    fn is_healthy(&self) -> bool {
        self.state().connections > 0
    }
}

impl Deref for DieselPool {
    type Target = ConnectionPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

pub type DieselPooledConn = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub fn oneoff_connection_with_config(
    config: &config::DatabasePools,
) -> ConnectionResult<PgConnection> {
    let url = connection_url(config, config.primary.url.expose_secret());
    PgConnection::establish(&url)
}

pub fn oneoff_connection() -> anyhow::Result<PgConnection> {
    let config = config::DatabasePools::full_from_environment(&config::Base::from_environment()?)?;
    oneoff_connection_with_config(&config).map_err(Into::into)
}

pub fn connection_url(config: &config::DatabasePools, url: &str) -> String {
    let mut url = Url::parse(url).expect("Invalid database URL");

    if config.enforce_tls {
        maybe_append_url_param(&mut url, "sslmode", "require");
    }

    // Configure the time it takes for diesel to return an error when there is full packet loss
    // between the application and the database.
    maybe_append_url_param(
        &mut url,
        "tcp_user_timeout",
        &config.tcp_timeout_ms.to_string(),
    );

    url.into()
}

fn maybe_append_url_param(url: &mut Url, key: &str, value: &str) {
    if !url.query_pairs().any(|(k, _)| k == key) {
        url.query_pairs_mut().append_pair(key, value);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConnectionConfig {
    pub statement_timeout: Duration,
    pub read_only: bool,
}

impl ConnectionConfig {
    fn apply(&self, conn: &mut PgConnection) -> QueryResult<()> {
        let statement_timeout = self.statement_timeout.as_millis();
        diesel::sql_query(format!("SET statement_timeout = {statement_timeout}")).execute(conn)?;

        if self.read_only {
            diesel::sql_query("SET default_transaction_read_only = 't'").execute(conn)?;
        }

        Ok(())
    }
}

impl CustomizeConnection<PgConnection, r2d2::Error> for ConnectionConfig {
    fn on_acquire(&self, conn: &mut PgConnection) -> Result<(), r2d2::Error> {
        self.apply(conn).map_err(r2d2::Error::QueryError)
    }
}

impl From<ConnectionConfig> for Hook {
    fn from(config: ConnectionConfig) -> Self {
        Hook::async_fn(move |conn, _| {
            Box::pin(async move {
                conn.interact(move |conn| config.apply(conn))
                    .await
                    .map_err(|err| HookError::Message(err.to_string()))?
                    .map_err(|err| HookError::Message(err.to_string()))
            })
        })
    }
}

#[derive(Debug, Error)]
pub enum PoolError {
    #[error(transparent)]
    R2D2(#[from] r2d2::PoolError),
    #[error("unhealthy database pool")]
    UnhealthyPool,
}
