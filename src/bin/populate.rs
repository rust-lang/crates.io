// Populate a set of dummy download statistics for a specific version in the
// database.
//
// Usage:
//      cargo run --bin populate version_id1 version_id2 ...

#![deny(warnings)]

extern crate cargo_registry;
extern crate chrono;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;
extern crate rand;

use chrono::{Duration, NaiveDate, Utc};
use diesel::prelude::*;
use rand::{Rng, StdRng};
use std::env;

use cargo_registry::schema::version_downloads;

fn main() {
    let conn = cargo_registry::db::connect_now().unwrap();
    conn.transaction(|| update(&conn)).unwrap();
}

fn update(conn: &PgConnection) -> QueryResult<()> {
    let ids = env::args()
        .skip(1)
        .filter_map(|arg| arg.parse::<i32>().ok());
    for id in ids {
        let mut rng = StdRng::new().unwrap();
        let mut dls = rng.gen_range(5000i32, 10000);

        for day in 0..90 {
            let moment = Utc::now().date().naive_utc() + Duration::days(-day);
            dls += rng.gen_range(-100, 100);

            let version_download = VersionDownload {
                version_id: id,
                downloads: dls,
                date: moment,
            };
            diesel::insert(&version_download)
                .into(version_downloads::table)
                .execute(conn)?;
        }
    }
    Ok(())
}

#[derive(Insertable)]
#[table_name = "version_downloads"]
struct VersionDownload {
    version_id: i32,
    downloads: i32,
    date: NaiveDate,
}
