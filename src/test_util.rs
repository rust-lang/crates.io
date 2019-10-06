#![cfg(test)]

use diesel::prelude::*;

pub fn pg_connection_no_transaction() -> PgConnection {
    let database_url =
        dotenv::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL must be set to run tests");
    PgConnection::establish(&database_url).unwrap()
}

pub fn pg_connection() -> PgConnection {
    let conn = pg_connection_no_transaction();
    conn.begin_test_transaction().unwrap();
    conn
}
