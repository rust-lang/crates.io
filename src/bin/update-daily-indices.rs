// Creates any partial indexes that need to be updated daily
//
// Should be run at just after midnight UTC on a daily basis

#![deny(warnings)]

extern crate cargo_registry;
#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel::select;

fn main() {
    let conn = cargo_registry::db::connect_now().unwrap();
    make_indices(&conn).unwrap();
}

no_arg_sql_function!(refresh_todays_recent_crate_downloads, ());
fn make_indices(conn: &PgConnection) -> QueryResult<()> {
    select(refresh_todays_recent_crate_downloads).execute(conn)?;
    Ok(())
}
