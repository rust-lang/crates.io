#![warn(clippy::all, rust_2018_idioms)]
#![deny(warnings)]

use cargo_registry::schema::*;
use cargo_registry::util::errors::*;
use cargo_registry::*;
use chrono::*;
use diesel::dsl::min;
use diesel::prelude::*;
use std::thread;
use std::time::Duration;

fn main() -> AppResult<()> {
    let conn = db::connect_now()?;
    let mut date = version_downloads::table
        .select(min(version_downloads::date))
        .get_result::<Option<NaiveDate>>(&conn)?
        .expect("Cannot run on an empty table");
    let today = Utc::today().naive_utc();

    while date <= today {
        println!("Backfilling {}", date);
        version_downloads::table
            .select((
                version_downloads::version_id,
                version_downloads::downloads,
                version_downloads::counted,
                version_downloads::date,
            ))
            .filter(version_downloads::date.eq(date))
            .insert_into(version_downloads_part::table)
            .on_conflict_do_nothing()
            .execute(&conn)?;
        date = date.succ();
        thread::sleep(Duration::from_millis(100))
    }

    let (new_downloads, old_downloads) = diesel::select((
        version_downloads::table.count().single_value(),
        version_downloads_part::table.count().single_value(),
    ))
    .get_result::<(Option<i64>, Option<i64>)>(&conn)?;
    assert_eq!(
        new_downloads, old_downloads,
        "download counts do not match after backfilling!"
    );

    Ok(())
}
