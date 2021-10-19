use crate::{db, schema::version_downloads};

use diesel::prelude::*;
use rand::{thread_rng, Rng};

#[derive(clap::Parser, Debug)]
#[clap(
    name = "populate",
    about = "Populate a set of dummy download statistics for a specific version in the database."
)]
pub struct Opts {
    #[clap(required = true)]
    version_ids: Vec<i32>,
}

pub fn run(opts: Opts) {
    let conn = db::connect_now().unwrap();
    conn.transaction(|| update(opts, &conn)).unwrap();
}

fn update(opts: Opts, conn: &PgConnection) -> QueryResult<()> {
    use diesel::dsl::*;

    for id in opts.version_ids {
        let mut rng = thread_rng();
        let mut dls = rng.gen_range(5_000i32..10_000);

        for day in 0..90 {
            dls += rng.gen_range(-100..100);

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
