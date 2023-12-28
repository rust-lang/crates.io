#![cfg(test)]

use crates_io_test_db::TestDatabase;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};

pub fn test_db_connection() -> (
    TestDatabase,
    PooledConnection<ConnectionManager<PgConnection>>,
) {
    let test_db = TestDatabase::new();
    let conn = test_db.connect();
    (test_db, conn)
}
