use conduit::RequestExt;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, CustomizeConnection};
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use prometheus::Histogram;
use std::sync::Arc;
use std::{ops::Deref, time::Duration};
use thiserror::Error;
use url::Url;

use crate::config;
use crate::middleware::app::RequestApp;

#[derive(Clone)]
pub enum DieselPool {
    Pool {
        pool: r2d2::Pool<ConnectionManager<PgConnection>>,
        time_to_obtain_connection_metric: Histogram,
    },
    Test(Arc<ReentrantMutex<PgConnection>>),
}

impl DieselPool {
    pub(crate) fn new(
        url: &str,
        config: &config::DatabasePools,
        r2d2_config: r2d2::Builder<ConnectionManager<PgConnection>>,
        time_to_obtain_connection_metric: Histogram,
    ) -> Result<DieselPool, PoolError> {
        let manager = ConnectionManager::new(connection_url(config, url));

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
        let pool = DieselPool::Pool {
            pool: r2d2_config.build_unchecked(manager),
            time_to_obtain_connection_metric,
        };
        match pool.wait_until_healthy(Duration::from_secs(5)) {
            Ok(()) => {}
            Err(PoolError::UnhealthyPool) => {}
            Err(err) => return Err(err),
        }

        Ok(pool)
    }

    pub(crate) fn new_test(config: &config::DatabasePools, url: &str) -> DieselPool {
        let conn = PgConnection::establish(&connection_url(config, url))
            .expect("failed to establish connection");
        conn.begin_test_transaction()
            .expect("failed to begin test transaction");
        DieselPool::Test(Arc::new(ReentrantMutex::new(conn)))
    }

    pub fn get(&self) -> Result<DieselPooledConn<'_>, PoolError> {
        match self {
            DieselPool::Pool {
                pool,
                time_to_obtain_connection_metric,
            } => time_to_obtain_connection_metric.observe_closure_duration(|| {
                if let Some(conn) = pool.try_get() {
                    Ok(DieselPooledConn::Pool(conn))
                } else if !self.is_healthy() {
                    Err(PoolError::UnhealthyPool)
                } else {
                    Ok(DieselPooledConn::Pool(pool.get()?))
                }
            }),
            DieselPool::Test(conn) => Ok(DieselPooledConn::Test(conn.lock())),
        }
    }

    pub fn state(&self) -> PoolState {
        match self {
            DieselPool::Pool { pool, .. } => {
                let state = pool.state();
                PoolState {
                    connections: state.connections,
                    idle_connections: state.idle_connections,
                }
            }
            DieselPool::Test(_) => PoolState {
                connections: 0,
                idle_connections: 0,
            },
        }
    }

    pub fn wait_until_healthy(&self, timeout: Duration) -> Result<(), PoolError> {
        match self {
            DieselPool::Pool { pool, .. } => match pool.get_timeout(timeout) {
                Ok(_) => Ok(()),
                Err(_) if !self.is_healthy() => Err(PoolError::UnhealthyPool),
                Err(err) => Err(PoolError::R2D2(err)),
            },
            DieselPool::Test(_) => Ok(()),
        }
    }

    fn is_healthy(&self) -> bool {
        self.state().connections > 0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PoolState {
    pub connections: u32,
    pub idle_connections: u32,
}

pub enum DieselPooledConn<'a> {
    Pool(r2d2::PooledConnection<ConnectionManager<PgConnection>>),
    Test(ReentrantMutexGuard<'a, PgConnection>),
}

impl Deref for DieselPooledConn<'_> {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        match self {
            DieselPooledConn::Pool(conn) => conn.deref(),
            DieselPooledConn::Test(conn) => conn.deref(),
        }
    }
}

pub fn oneoff_connection_with_config(
    config: &config::DatabasePools,
) -> ConnectionResult<PgConnection> {
    let url = connection_url(config, &config.primary.url);
    PgConnection::establish(&url)
}

pub fn oneoff_connection() -> ConnectionResult<PgConnection> {
    let config = config::DatabasePools::full_from_environment(&config::Base::from_environment());
    oneoff_connection_with_config(&config)
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

pub trait RequestTransaction {
    /// Obtain a read/write database connection from the primary pool
    fn db_write(&self) -> Result<DieselPooledConn<'_>, PoolError>;

    /// Obtain a readonly database connection from the replica pool
    ///
    /// If the replica pool is disabled or unavailable, the primary pool is used instead.
    fn db_read(&self) -> Result<DieselPooledConn<'_>, PoolError>;

    /// Obtain a readonly database connection from the primary pool
    ///
    /// If the primary pool is unavailable, the replica pool is used instead, if not disabled.
    fn db_read_prefer_primary(&self) -> Result<DieselPooledConn<'_>, PoolError>;
}

impl<T: RequestExt + ?Sized> RequestTransaction for T {
    fn db_write(&self) -> Result<DieselPooledConn<'_>, PoolError> {
        self.app().primary_database.get()
    }

    fn db_read(&self) -> Result<DieselPooledConn<'_>, PoolError> {
        let read_only_pool = self.app().read_only_replica_database.as_ref();
        match read_only_pool.map(|pool| pool.get()) {
            // Replica is available
            Some(Ok(connection)) => Ok(connection),

            // Replica is not available, but primary might be available
            Some(Err(PoolError::UnhealthyPool)) => self.app().primary_database.get(),

            // Replica failed
            Some(Err(error)) => Err(error),

            // Replica is disabled, but primary might be available
            None => self.app().primary_database.get(),
        }
    }

    fn db_read_prefer_primary(&self) -> Result<DieselPooledConn<'_>, PoolError> {
        match (
            self.app().primary_database.get(),
            &self.app().read_only_replica_database,
        ) {
            // Primary is available
            (Ok(connection), _) => Ok(connection),

            // Primary is not available, but replica might be available
            (Err(PoolError::UnhealthyPool), Some(read_only_pool)) => read_only_pool.get(),

            // Primary failed and replica is disabled
            (Err(error), None) => Err(error),

            // Primary failed
            (Err(error), _) => Err(error),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConnectionConfig {
    pub statement_timeout: u64,
    pub read_only: bool,
}

impl CustomizeConnection<PgConnection, r2d2::Error> for ConnectionConfig {
    fn on_acquire(&self, conn: &mut PgConnection) -> Result<(), r2d2::Error> {
        use diesel::sql_query;

        sql_query(format!(
            "SET statement_timeout = {}",
            self.statement_timeout * 1000
        ))
        .execute(conn)
        .map_err(r2d2::Error::QueryError)?;
        if self.read_only {
            sql_query("SET default_transaction_read_only = 't'")
                .execute(conn)
                .map_err(r2d2::Error::QueryError)?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn test_conn() -> PgConnection {
    let conn = PgConnection::establish(&crate::env("TEST_DATABASE_URL")).unwrap();
    conn.begin_test_transaction().unwrap();
    conn
}

#[derive(Debug, Error)]
pub enum PoolError {
    #[error(transparent)]
    R2D2(#[from] r2d2::PoolError),
    #[error("unhealthy database pool")]
    UnhealthyPool,
}
