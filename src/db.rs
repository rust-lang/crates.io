use std::env;

use conduit::Request;
use diesel::prelude::{ConnectionResult, PgConnection};
use r2d2;
use r2d2_diesel::{self, ConnectionManager};
use url::Url;

use app::RequestApp;
use util::CargoResult;

pub type DieselPool = r2d2::Pool<ConnectionManager<PgConnection>>;
type DieselPooledConn = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub fn connect_now() -> ConnectionResult<PgConnection> {
    use diesel::Connection;
    let mut url = Url::parse(&::env("DATABASE_URL")).expect("Invalid database URL");
    if env::var("HEROKU").is_ok() && !url.query_pairs().any(|(k, _)| k == "sslmode") {
        url.query_pairs_mut().append_pair("sslmode", "require");
    }
    PgConnection::establish(&url.to_string())
}

pub fn diesel_pool(
    url: &str,
    config: r2d2::Config<PgConnection, r2d2_diesel::Error>,
) -> DieselPool {
    let mut url = Url::parse(url).expect("Invalid database URL");
    if env::var("HEROKU").is_ok() && !url.query_pairs().any(|(k, _)| k == "sslmode") {
        url.query_pairs_mut().append_pair("sslmode", "require");
    }
    let manager = ConnectionManager::new(url.into_string());
    r2d2::Pool::new(config, manager).unwrap()
}

pub trait RequestTransaction {
    /// Return the lazily initialized postgres connection for this request.
    ///
    /// The connection will live for the lifetime of the request.
    fn db_conn(&self) -> CargoResult<DieselPooledConn>;
}

impl<T: Request + ?Sized> RequestTransaction for T {
    fn db_conn(&self) -> CargoResult<DieselPooledConn> {
        self.app().diesel_database.get().map_err(Into::into)
    }
}
