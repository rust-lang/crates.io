#![cfg(test)]

use crates_io_env_vars::required_var;
use diesel::prelude::*;

pub fn pg_connection_no_transaction() -> PgConnection {
    let database_url = required_var("TEST_DATABASE_URL").unwrap();
    PgConnection::establish(&database_url).unwrap()
}

pub fn pg_connection() -> PgConnection {
    let mut conn = pg_connection_no_transaction();
    conn.begin_test_transaction().unwrap();
    conn
}
