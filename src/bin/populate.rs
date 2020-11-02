#![warn(clippy::all, rust_2018_idioms)]

use cargo_registry::{db, schema::version_downloads};

use clap::Clap;
use diesel::prelude::*;
use rand::{thread_rng, Rng};

#[derive(Clap, Debug)]
#[clap(
    name = "populate",
    about = "Populate a set of dummy download statistics for a specific version in the database."
)]
struct Opts {
    #[clap(required = true)]
    version_ids: Vec<i32>,
}

fn main() {
    let conn = db::connect_now().unwrap();
    conn.transaction(|| update(&conn)).unwrap();
}

fn update(conn: &PgConnection) -> QueryResult<()> {
    use diesel::dsl::*;

    let opts: Opts = Opts::parse();

    for id in opts.version_ids {
        let mut rng = thread_rng();
        let mut dls = rng.gen_range(5_000i32, 10_000);

        for day in 0..90 {
            dls += rng.gen_range(-100, 100);

            diesel::insert_into(version_downloads::table)
                .values((
                    version_downloads::version_id.eq(id),
                    version_downloads::downloads.eq(dls),
                    version_downloads::date.eq(date(now - day.days())),
                ))
                .execute(conn)?;
        }
    }
    Ok(())
}
