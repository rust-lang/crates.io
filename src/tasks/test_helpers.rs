use crate::env;
use diesel::prelude::*;

pub(crate) fn conn() -> PgConnection {
    let conn = PgConnection::establish(&env("TEST_DATABASE_URL")).unwrap();
    conn.begin_test_transaction().unwrap();
    conn
}
