use conduit::RequestExt;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, CustomizeConnection};
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use std::ops::Deref;
use std::sync::Arc;
use url::Url;

use crate::middleware::app::RequestApp;
use crate::Env;

#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub enum DieselPool {
    Pool(r2d2::Pool<ConnectionManager<PgConnection>>),
    Test(Arc<ReentrantMutex<PgConnection>>),
}

impl DieselPool {
    pub fn get(&self) -> Result<DieselPooledConn<'_>, r2d2::PoolError> {
        match self {
            DieselPool::Pool(pool) => Ok(DieselPooledConn::Pool(pool.get()?)),
            DieselPool::Test(conn) => Ok(DieselPooledConn::Test(conn.lock())),
        }
    }

    pub fn state(&self) -> PoolState {
        match self {
            DieselPool::Pool(pool) => {
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

    fn test_conn(conn: PgConnection) -> Self {
        DieselPool::Test(Arc::new(ReentrantMutex::new(conn)))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PoolState {
    pub connections: u32,
    pub idle_connections: u32,
}

#[allow(missing_debug_implementations)]
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
    url.into_string()
}

pub fn diesel_pool(
    url: &str,
    env: Env,
    config: r2d2::Builder<ConnectionManager<PgConnection>>,
) -> DieselPool {
    let url = connection_url(url);
    if env == Env::Test {
        let conn = PgConnection::establish(&url).expect("failed to establish connection");
        DieselPool::test_conn(conn)
    } else {
        let manager = ConnectionManager::new(url);
        DieselPool::Pool(config.build(manager).unwrap())
    }
}

pub trait RequestTransaction {
    /// Obtain a read/write database connection from the primary pool
    fn db_conn(&self) -> Result<DieselPooledConn<'_>, r2d2::PoolError>;

    /// Obtain a readonly database connection from the replica pool
    ///
    /// If there is no replica pool, the primary pool is used instead.
    fn db_read_only(&self) -> Result<DieselPooledConn<'_>, r2d2::PoolError>;
}

impl<T: RequestExt + ?Sized> RequestTransaction for T {
    fn db_conn(&self) -> Result<DieselPooledConn<'_>, r2d2::PoolError> {
        self.app().primary_database.get().map_err(Into::into)
    }

    fn db_read_only(&self) -> Result<DieselPooledConn<'_>, r2d2::PoolError> {
        match &self.app().read_only_replica_database {
            Some(pool) => pool.get().map_err(Into::into),
            None => self.app().primary_database.get().map_err(Into::into),
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
