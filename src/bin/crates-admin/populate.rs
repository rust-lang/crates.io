use crates_io::{db, schema::version_downloads};

use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use rand::RngExt;

#[derive(clap::Parser, Debug)]
#[command(
    name = "populate",
    about = "Populate a set of dummy download statistics for a specific version in the database."
)]
pub struct Opts {
    #[arg(required = true)]
    version_ids: Vec<i32>,
}

pub async fn run(opts: Opts) -> anyhow::Result<()> {
    let mut conn = db::oneoff_connection().await?;
    conn.transaction(|conn| update(opts, conn).scope_boxed())
        .await?;
    Ok(())
}

async fn update(opts: Opts, conn: &mut AsyncPgConnection) -> QueryResult<()> {
    use diesel::dsl::*;

    for id in opts.version_ids {
        let mut dls = rand::rng().random_range(5_000i32..10_000);

        for day in 0..90 {
            dls += rand::rng().random_range(-100..100);

            diesel::insert_into(version_downloads::table)
                .values((
                    version_downloads::version_id.eq(id),
                    version_downloads::downloads.eq(dls),
                    version_downloads::date.eq(date(now - day.days())),
                ))
                .execute(conn)
                .await?;
        }
    }
    Ok(())
}
