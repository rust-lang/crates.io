use crate::schema::version_downloads;
use crate::tasks::spawn_blocking;
use crate::util::diesel::Conn;
use crate::worker::Environment;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize)]
pub struct UpdateDownloads;

impl BackgroundJob for UpdateDownloads {
    const JOB_NAME: &'static str = "update_downloads";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        let conn = env.deadpool.get().await?;
        spawn_blocking(move || {
            let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();
            Ok(update(conn)?)
        })
        .await
    }
}

fn update(conn: &mut impl Conn) -> QueryResult<()> {
    use diesel::dsl::now;
    use diesel::select;

    info!("Updating versions…");

    // After 45 minutes, we stop the batch updating process to avoid
    // triggering the long-running job alert. 15 minutes later, the job
    // will be started again by our cron service anyway.
    const TIME_LIMIT: Duration = Duration::from_secs(45 * 60);

    // We update the `downloads` columns in batches to a) avoid the
    // back and forth between the application and the database and b) avoid
    // holding locks on any of the involved tables for too long.
    const BATCH_SIZE: i64 = 5_000;

    let start_time = Instant::now();
    loop {
        let update_count = batch_update(BATCH_SIZE, conn)?;
        info!("Updated {update_count} versions");
        if update_count < BATCH_SIZE {
            break;
        }

        if start_time.elapsed() > TIME_LIMIT {
            info!("Time limit reached, stopping batch update");
            break;
        }
    }

    info!("Finished updating versions");

    // Anything older than 24 hours ago will be frozen and will not be queried
    // against again.
    diesel::update(version_downloads::table)
        .set(version_downloads::processed.eq(true))
        .filter(version_downloads::date.lt(diesel::dsl::date(now)))
        .filter(version_downloads::downloads.eq(version_downloads::counted))
        .filter(version_downloads::processed.eq(false))
        .execute(conn)?;
    info!("Finished freezing old version_downloads");

    define_sql_function!(fn refresh_recent_crate_downloads());
    select(refresh_recent_crate_downloads()).execute(conn)?;
    info!("Finished running refresh_recent_crate_downloads");

    Ok(())
}

#[instrument(skip_all)]
fn batch_update(batch_size: i64, conn: &mut impl Conn) -> QueryResult<i64> {
    table! {
        /// Imaginary table to make Diesel happy when using the `sql_query` function.
        sql_query_results (count) {
            count -> BigInt,
        }
    }

    /// A helper struct for the result of the query.
    ///
    /// The result of `sql_query` can not be a tuple, so we have to define a
    /// proper struct for the result.
    #[derive(QueryableByName)]
    struct SqlQueryResult {
        count: i64,
    }

    let result = diesel::sql_query(include_str!("update_metadata.sql"))
        .bind::<BigInt, _>(batch_size)
        .get_result::<SqlQueryResult>(conn)?;

    Ok(result.count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::email::Emails;
    use crate::models::{Crate, NewCrate, NewUser, NewVersion, User, Version};
    use crate::schema::{crate_downloads, crates, versions};
    use crate::test_util::test_db_connection;

    fn user(conn: &mut impl Conn) -> User {
        NewUser::new(2, "login", None, None, "access_token")
            .create_or_update(None, &Emails::new_in_memory(), conn)
            .unwrap()
    }

    fn crate_and_version(conn: &mut impl Conn, user_id: i32) -> (Crate, Version) {
        let krate = NewCrate {
            name: "foo",
            ..Default::default()
        }
        .create(conn, user_id)
        .unwrap();

        let version = NewVersion::builder(krate.id, "1.0.0")
            .published_by(user_id)
            .checksum("0000000000000000000000000000000000000000000000000000000000000000")
            .build();

        let version = version.save(conn, "someone@example.com").unwrap();
        (krate, version)
    }

    #[test]
    fn increment() {
        use diesel::dsl::*;

        let (_test_db, conn) = &mut test_db_connection();
        let user = user(conn);
        let (krate, version) = crate_and_version(conn, user.id);
        insert_into(version_downloads::table)
            .values(version_downloads::version_id.eq(version.id))
            .execute(conn)
            .unwrap();
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::date.eq(date(now - 1.day())),
                version_downloads::processed.eq(true),
            ))
            .execute(conn)
            .unwrap();

        super::update(conn).unwrap();

        let version_downloads = versions::table
            .find(version.id)
            .select(versions::downloads)
            .first(conn);
        assert_eq!(version_downloads, Ok(1));

        let crate_downloads = crate_downloads::table
            .find(krate.id)
            .select(crate_downloads::downloads)
            .first(conn);
        assert_eq!(crate_downloads, Ok(1));

        super::update(conn).unwrap();

        let version_downloads = versions::table
            .find(version.id)
            .select(versions::downloads)
            .first(conn);
        assert_eq!(version_downloads, Ok(1));
    }

    #[test]
    fn set_processed_true() {
        use diesel::dsl::*;

        let (_test_db, conn) = &mut test_db_connection();
        let user = user(conn);
        let (_, version) = crate_and_version(conn, user.id);
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::downloads.eq(2),
                version_downloads::counted.eq(2),
                version_downloads::date.eq(date(now - 2.days())),
                version_downloads::processed.eq(false),
            ))
            .execute(conn)
            .unwrap();
        super::update(conn).unwrap();
        let processed = version_downloads::table
            .filter(version_downloads::version_id.eq(version.id))
            .select(version_downloads::processed)
            .first(conn);
        assert_eq!(processed, Ok(true));
    }

    #[test]
    fn dont_process_recent_row() {
        use diesel::dsl::*;
        let (_test_db, conn) = &mut test_db_connection();
        let user = user(conn);
        let (_, version) = crate_and_version(conn, user.id);
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::downloads.eq(2),
                version_downloads::counted.eq(2),
                version_downloads::date.eq(date(now)),
                version_downloads::processed.eq(false),
            ))
            .execute(conn)
            .unwrap();
        super::update(conn).unwrap();
        let processed = version_downloads::table
            .filter(version_downloads::version_id.eq(version.id))
            .select(version_downloads::processed)
            .first(conn);
        assert_eq!(processed, Ok(false));
    }

    #[test]
    fn increment_a_little() {
        use diesel::dsl::*;
        use diesel::update;

        let (_test_db, conn) = &mut test_db_connection();
        let user = user(conn);
        let (krate, version) = crate_and_version(conn, user.id);
        update(versions::table)
            .set(versions::updated_at.eq(now - 2.hours()))
            .execute(conn)
            .unwrap();
        update(crates::table)
            .set(crates::updated_at.eq(now - 2.hours()))
            .execute(conn)
            .unwrap();
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::downloads.eq(2),
                version_downloads::counted.eq(1),
                version_downloads::date.eq(date(now)),
                version_downloads::processed.eq(false),
            ))
            .execute(conn)
            .unwrap();
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::date.eq(date(now - 1.day())),
            ))
            .execute(conn)
            .unwrap();

        let version_before: Version = versions::table.find(version.id).first(conn).unwrap();
        let krate_before: Crate = Crate::all()
            .filter(crates::id.eq(krate.id))
            .first(conn)
            .unwrap();

        super::update(conn).unwrap();

        let version2: Version = versions::table.find(version.id).first(conn).unwrap();
        assert_eq!(version2.downloads, 2);
        assert_eq!(version2.updated_at, version_before.updated_at);

        let krate2: Crate = Crate::all()
            .filter(crates::id.eq(krate.id))
            .first(conn)
            .unwrap();
        assert_eq!(krate2.updated_at, krate_before.updated_at);

        let krate2_downloads: i64 = crate_downloads::table
            .find(krate.id)
            .select(crate_downloads::downloads)
            .first(conn)
            .unwrap();
        assert_eq!(krate2_downloads, 2);

        super::update(conn).unwrap();

        let version3: Version = versions::table.find(version.id).first(conn).unwrap();
        assert_eq!(version3.downloads, 2);
    }

    #[test]
    fn set_processed_no_set_updated_at() {
        use diesel::dsl::*;
        use diesel::update;

        let (_test_db, mut conn) = test_db_connection();

        // This test is using a transaction to ensure `now` is the same for all
        // queries within this test.
        conn.begin_test_transaction().unwrap();

        let conn = &mut conn;

        let user = user(conn);
        let (_, version) = crate_and_version(conn, user.id);
        update(versions::table)
            .set(versions::updated_at.eq(now - 2.days()))
            .execute(conn)
            .unwrap();
        update(crates::table)
            .set(crates::updated_at.eq(now - 2.days()))
            .execute(conn)
            .unwrap();
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::downloads.eq(2),
                version_downloads::counted.eq(2),
                version_downloads::date.eq(date(now - 2.days())),
                version_downloads::processed.eq(false),
            ))
            .execute(conn)
            .unwrap();

        super::update(conn).unwrap();
        let versions_changed = versions::table
            .select(versions::updated_at.ne(now - 2.days()))
            .get_result(conn);
        let crates_changed = crates::table
            .select(crates::updated_at.ne(now - 2.days()))
            .get_result(conn);
        assert_eq!(versions_changed, Ok(false));
        assert_eq!(crates_changed, Ok(false));
    }
}
