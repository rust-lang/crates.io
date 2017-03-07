#![deny(warnings)]

#[macro_use]
extern crate diesel;
extern crate dotenv;

use diesel::*;
use diesel::migrations::setup_database;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

table! {
    information_schema.tables (table_name) {
        table_name -> Text,
    }
}

fn main() {
    let _ = dotenv();
    // This code will eventually break, as it relies on internal details of Diesel.
    // If this function stops working, it's probably better to just delete it.
    let conn = PgConnection::establish(&env::var("DATABASE_URL").unwrap()).unwrap();

    if !table_exists("__diesel_schema_migrations", &conn) {
        setup_database(&conn).unwrap();
    }

    if table_exists("schema_migrations", &conn) {
        conn.execute("INSERT INTO __diesel_schema_migrations (
            SELECT version::text AS version, CURRENT_TIMESTAMP as run_on
                FROM schema_migrations
        ) ON CONFLICT DO NOTHING").unwrap();
    }

    println!("The `migrate` binary is no longer used. Use `diesel migration run` \
              and `diesel migration revert` instead.");
}

fn table_exists(target: &str, conn: &PgConnection) -> bool {
    use self::tables::dsl::*;
    use diesel::expression::dsl::exists;

    let table_query = tables.filter(table_name.eq(target));
    select(exists(table_query)).get_result(conn).unwrap()
}
