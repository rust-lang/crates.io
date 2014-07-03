use pg;
use pg::PostgresConnection;
use pg::pool::PostgresConnectionPool;

use packages;
use user;

fn location() -> String {
    "postgres://postgres:@localhost/cargo.io".to_string()
}

pub fn pool() -> PostgresConnectionPool {
    PostgresConnectionPool::new(location().as_slice(), pg::NoSsl, 5).unwrap()
}

pub fn setup(conn: &PostgresConnection) {
    packages::setup(conn);
    user::setup(conn);
}
