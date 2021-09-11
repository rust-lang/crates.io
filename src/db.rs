use conduit::RequestExt;
use diesel::prelude::*;
use diesel::r2d2::{self, event::HandleEvent, ConnectionManager, CustomizeConnection};
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use prometheus::{Histogram, IntCounter};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::{ops::Deref, time::Duration};
use thiserror::Error;
use url::Url;

use crate::middleware::app::RequestApp;

#[derive(Clone)]
pub enum DieselPool {
    Pool {
        pool: r2d2::Pool<ConnectionManager<PgConnection>>,
        state: Arc<StateTracker>,
        used_conns_metric: Histogram,
        time_to_obtain_connection_metric: Histogram,
    },
    Test(Arc<ReentrantMutex<PgConnection>>),
}

impl DieselPool {
    pub(crate) fn new(
        url: &str,
        mut config: r2d2::Builder<ConnectionManager<PgConnection>>,
        used_conns_metric: Histogram,
        time_to_obtain_connection_metric: Histogram,
        timeouts_when_obtaining_connections: IntCounter,
    ) -> Result<DieselPool, PoolError> {
        let manager = ConnectionManager::new(connection_url(url));

        // Track the pool state using r2d2's event handler.
        let state = Arc::new(StateTracker::new(timeouts_when_obtaining_connections));
        config = config.event_handler(Box::new(StateEventHandler(state.clone())));

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
            pool: config.build_unchecked(manager),
            state,
            used_conns_metric,
            time_to_obtain_connection_metric,
        };
        match pool.wait_until_healthy(Duration::from_secs(5)) {
            Ok(()) => {}
            Err(PoolError::UnhealthyPool) => {}
            Err(err) => return Err(err),
        }

        Ok(pool)
    }

    pub(crate) fn new_test(url: &str) -> DieselPool {
        let conn =
            PgConnection::establish(&connection_url(url)).expect("failed to establish connection");
        conn.begin_test_transaction()
            .expect("failed to begin test transaction");
        DieselPool::Test(Arc::new(ReentrantMutex::new(conn)))
    }

    pub fn get(&self) -> Result<DieselPooledConn<'_>, PoolError> {
        match self {
            DieselPool::Pool {
                pool,
                state,
                used_conns_metric,
                time_to_obtain_connection_metric,
            } => time_to_obtain_connection_metric.observe_closure_duration(|| {
                // Record the number of used connections before obtaining the current one.
                used_conns_metric.observe(state.used_connections.load(Ordering::SeqCst) as f64);

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
            DieselPool::Pool { state, .. } => state.pool_state(),
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

unsafe impl<'a> Send for DieselPooledConn<'a> {}

impl Deref for DieselPooledConn<'_> {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        match self {
            DieselPooledConn::Pool(conn) => conn.deref(),
            DieselPooledConn::Test(conn) => conn.deref(),
        }
    }
}

pub fn connect_now() -> ConnectionResult<PgConnection> {
    let url = connection_url(&crate::env("DATABASE_URL"));
    PgConnection::establish(&url)
}

pub fn connection_url(url: &str) -> String {
    let mut url = Url::parse(url).expect("Invalid database URL");
    if dotenv::var("HEROKU").is_ok() && !url.query_pairs().any(|(k, _)| k == "sslmode") {
        url.query_pairs_mut().append_pair("sslmode", "require");
    }
    url.into()
}

pub trait RequestTransaction {
    /// Obtain a read/write database connection from the primary pool
    fn db_conn(&self) -> Result<DieselPooledConn<'_>, PoolError>;

    /// Obtain a readonly database connection from the replica pool
    ///
    /// If there is no replica pool, the primary pool is used instead.
    fn db_read_only(&self) -> Result<DieselPooledConn<'_>, PoolError>;
}

impl<T: RequestExt + ?Sized> RequestTransaction for T {
    fn db_conn(&self) -> Result<DieselPooledConn<'_>, PoolError> {
        self.app().primary_database.get()
    }

    fn db_read_only(&self) -> Result<DieselPooledConn<'_>, PoolError> {
        match &self.app().read_only_replica_database {
            Some(pool) => pool.get(),
            None => self.app().primary_database.get(),
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

/// As r2d2's state is behind a mutex, we had operational issues in the past trying to access it on
/// every request to populate metrics (a lot of requests started timing out).
///
/// This reimplements state tracking using atomics and r2d2's event handlers. Event handlers are
/// executed the same time a connection is checked in or out, without having to acquire another
/// mutex lock, so the performance should be higher than accessing the state managed by r2d2.
#[derive(Debug)]
pub struct StateTracker {
    open_connections: AtomicU32,
    used_connections: AtomicU32,
    timeouts_metric: IntCounter,
}

impl StateTracker {
    fn new(timeouts_metric: IntCounter) -> Self {
        StateTracker {
            open_connections: AtomicU32::new(0),
            used_connections: AtomicU32::new(0),
            timeouts_metric,
        }
    }

    fn pool_state(&self) -> PoolState {
        let connections = self.open_connections.load(Ordering::SeqCst);
        PoolState {
            connections,
            idle_connections: connections
                .saturating_sub(self.used_connections.load(Ordering::SeqCst)),
        }
    }
}

#[derive(Debug)]
struct StateEventHandler(Arc<StateTracker>);

impl HandleEvent for StateEventHandler {
    fn handle_acquire(&self, _: r2d2::event::AcquireEvent) {
        self.0.open_connections.fetch_add(1, Ordering::SeqCst);
    }

    fn handle_release(&self, _: r2d2::event::ReleaseEvent) {
        self.0.open_connections.fetch_sub(1, Ordering::SeqCst);
    }

    fn handle_checkout(&self, _: r2d2::event::CheckoutEvent) {
        self.0.used_connections.fetch_add(1, Ordering::SeqCst);
    }

    fn handle_checkin(&self, _: r2d2::event::CheckinEvent) {
        self.0.used_connections.fetch_sub(1, Ordering::SeqCst);
    }

    fn handle_timeout(&self, _: r2d2::event::TimeoutEvent) {
        self.0.timeouts_metric.inc();
    }
}
