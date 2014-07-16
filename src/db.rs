use pg;
use pg::{PostgresConnection, PostgresStatement, PostgresResult};
use pg::pool::PostgresConnectionPool;
use pg::types::ToSql;

use user;
use package;

fn location() -> String {
    "postgres://postgres:@localhost/cargo.io".to_string()
}

pub fn pool() -> PostgresConnectionPool {
    PostgresConnectionPool::new(location().as_slice(), pg::NoSsl, 5).unwrap()
}

pub fn setup(conn: &PostgresConnection) {
    user::setup(conn);
    package::setup(conn);
}

pub trait Connection {
    fn prepare<'a>(&'a self, query: &str) -> PostgresResult<PostgresStatement<'a>>;
    fn execute(&self, query: &str, params: &[&ToSql]) -> PostgresResult<uint>;
}

impl Connection for pg::PostgresConnection {
    fn prepare<'a>(&'a self, query: &str) -> PostgresResult<PostgresStatement<'a>> {
        self.prepare(query)
    }
    fn execute(&self, query: &str, params: &[&ToSql]) -> PostgresResult<uint> {
        self.execute(query, params)
    }
}

impl Connection for pg::pool::PooledPostgresConnection {
    fn prepare<'a>(&'a self, query: &str) -> PostgresResult<PostgresStatement<'a>> {
        self.prepare(query)
    }
    fn execute(&self, query: &str, params: &[&ToSql]) -> PostgresResult<uint> {
        self.execute(query, params)
    }
}

impl<'a> Connection for pg::PostgresTransaction<'a> {
    fn prepare<'a>(&'a self, query: &str) -> PostgresResult<PostgresStatement<'a>> {
        self.prepare(query)
    }
    fn execute(&self, query: &str, params: &[&ToSql]) -> PostgresResult<uint> {
        self.execute(query, params)
    }
}
