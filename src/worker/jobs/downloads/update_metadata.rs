use crate::schema::version_downloads;
use crate::worker::Environment;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize)]
pub struct UpdateDownloads;

impl BackgroundJob for UpdateDownloads {
    const JOB_NAME: &'static str = "update_downloads";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        let mut conn = env.deadpool.get().await?;
        Ok(update(&mut conn).await?)
    }
}

async fn update(conn: &mut AsyncPgConnection) -> QueryResult<()> {
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
        let update_count = batch_update(BATCH_SIZE, conn).await?;
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
        .execute(conn)
        .await?;
    info!("Finished freezing old version_downloads");

    define_sql_function!(fn refresh_recent_crate_downloads());

    select(refresh_recent_crate_downloads())
        .execute(conn)
        .await?;

    info!("Finished running refresh_recent_crate_downloads");

    Ok(())
}

#[instrument(skip_all)]
async fn batch_update(batch_size: i64, conn: &mut AsyncPgConnection) -> QueryResult<i64> {
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
        .get_result::<SqlQueryResult>(conn)
        .await?;

    Ok(result.count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Crate, NewCrate, NewUser, NewVersion, User, Version};
    use crate::schema::{crate_downloads, crates, users, versions};
    use crates_io_test_db::TestDatabase;
    use diesel_async::AsyncConnection;

    async fn user(conn: &mut AsyncPgConnection) -> User {
        let user = NewUser::new(2, "login", None, None, "access_token");
        diesel::insert_into(users::table)
            .values(user)
            .get_result(conn)
            .await
            .unwrap()
    }

    async fn crate_and_version(conn: &mut AsyncPgConnection, user_id: i32) -> (Crate, Version) {
        let krate = NewCrate {
            name: "foo",
            ..Default::default()
        }
        .create(conn, user_id)
        .await
        .unwrap();

        let version = NewVersion::builder(krate.id, "1.0.0")
            .published_by(user_id)
            .checksum("0000000000000000000000000000000000000000000000000000000000000000")
            .build();

        let version = version.save(conn, "someone@example.com").await.unwrap();
        (krate, version)
    }

    #[tokio::test]
    async fn increment() {
        use diesel::dsl::*;

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let user = user(&mut conn).await;
        let (krate, version) = crate_and_version(&mut conn, user.id).await;
        insert_into(version_downloads::table)
            .values(version_downloads::version_id.eq(version.id))
            .execute(&mut conn)
            .await
            .unwrap();
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::date.eq(date(now - 1.day())),
                version_downloads::processed.eq(true),
            ))
            .execute(&mut conn)
            .await
            .unwrap();

        super::update(&mut conn).await.unwrap();

        let version_downloads = versions::table
            .find(version.id)
            .select(versions::downloads)
            .first(&mut conn)
            .await;
        assert_eq!(version_downloads, Ok(1));

        let crate_downloads = crate_downloads::table
            .find(krate.id)
            .select(crate_downloads::downloads)
            .first(&mut conn)
            .await;
        assert_eq!(crate_downloads, Ok(1));

        super::update(&mut conn).await.unwrap();

        let version_downloads = versions::table
            .find(version.id)
            .select(versions::downloads)
            .first(&mut conn)
            .await;
        assert_eq!(version_downloads, Ok(1));
    }

    #[tokio::test]
    async fn set_processed_true() {
        use diesel::dsl::*;

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let user = user(&mut conn).await;
        let (_, version) = crate_and_version(&mut conn, user.id).await;
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::downloads.eq(2),
                version_downloads::counted.eq(2),
                version_downloads::date.eq(date(now - 2.days())),
                version_downloads::processed.eq(false),
            ))
            .execute(&mut conn)
            .await
            .unwrap();
        super::update(&mut conn).await.unwrap();
        let processed = version_downloads::table
            .filter(version_downloads::version_id.eq(version.id))
            .select(version_downloads::processed)
            .first(&mut conn)
            .await;
        assert_eq!(processed, Ok(true));
    }

    #[tokio::test]
    async fn dont_process_recent_row() {
        use diesel::dsl::*;
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let user = user(&mut conn).await;
        let (_, version) = crate_and_version(&mut conn, user.id).await;
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::downloads.eq(2),
                version_downloads::counted.eq(2),
                version_downloads::date.eq(date(now)),
                version_downloads::processed.eq(false),
            ))
            .execute(&mut conn)
            .await
            .unwrap();
        super::update(&mut conn).await.unwrap();
        let processed = version_downloads::table
            .filter(version_downloads::version_id.eq(version.id))
            .select(version_downloads::processed)
            .first(&mut conn)
            .await;
        assert_eq!(processed, Ok(false));
    }

    #[tokio::test]
    async fn increment_a_little() {
        use diesel::dsl::*;
        use diesel::update;

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let user = user(&mut conn).await;
        let (krate, version) = crate_and_version(&mut conn, user.id).await;
        update(versions::table)
            .set(versions::updated_at.eq(now - 2.hours()))
            .execute(&mut conn)
            .await
            .unwrap();
        update(crates::table)
            .set(crates::updated_at.eq(now - 2.hours()))
            .execute(&mut conn)
            .await
            .unwrap();
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::downloads.eq(2),
                version_downloads::counted.eq(1),
                version_downloads::date.eq(date(now)),
                version_downloads::processed.eq(false),
            ))
            .execute(&mut conn)
            .await
            .unwrap();
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::date.eq(date(now - 1.day())),
            ))
            .execute(&mut conn)
            .await
            .unwrap();

        let version_before: Version = versions::table
            .find(version.id)
            .first(&mut conn)
            .await
            .unwrap();
        let krate_before: Crate = Crate::all()
            .filter(crates::id.eq(krate.id))
            .first(&mut conn)
            .await
            .unwrap();

        super::update(&mut conn).await.unwrap();

        let version2: Version = versions::table
            .find(version.id)
            .first(&mut conn)
            .await
            .unwrap();
        assert_eq!(version2.downloads, 2);
        assert_eq!(version2.updated_at, version_before.updated_at);

        let krate2: Crate = Crate::all()
            .filter(crates::id.eq(krate.id))
            .first(&mut conn)
            .await
            .unwrap();
        assert_eq!(krate2.updated_at, krate_before.updated_at);

        let krate2_downloads: i64 = crate_downloads::table
            .find(krate.id)
            .select(crate_downloads::downloads)
            .first(&mut conn)
            .await
            .unwrap();
        assert_eq!(krate2_downloads, 2);

        super::update(&mut conn).await.unwrap();

        let version3: Version = versions::table
            .find(version.id)
            .first(&mut conn)
            .await
            .unwrap();
        assert_eq!(version3.downloads, 2);
    }

    #[tokio::test]
    async fn set_processed_no_set_updated_at() {
        use diesel::dsl::*;
        use diesel::update;

        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let user = user(&mut conn).await;
        let (_, version) = crate_and_version(&mut conn, user.id).await;

        // This test is using a transaction to ensure `now` is the same for all
        // queries within this test.
        conn.begin_test_transaction().await.unwrap();

        update(versions::table)
            .set(versions::updated_at.eq(now - 2.days()))
            .execute(&mut conn)
            .await
            .unwrap();
        update(crates::table)
            .set(crates::updated_at.eq(now - 2.days()))
            .execute(&mut conn)
            .await
            .unwrap();
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::downloads.eq(2),
                version_downloads::counted.eq(2),
                version_downloads::date.eq(date(now - 2.days())),
                version_downloads::processed.eq(false),
            ))
            .execute(&mut conn)
            .await
            .unwrap();

        super::update(&mut conn).await.unwrap();

        let versions_changed = versions::table
            .select(versions::updated_at.ne(now - 2.days()))
            .get_result(&mut conn)
            .await;
        let crates_changed = crates::table
            .select(crates::updated_at.ne(now - 2.days()))
            .get_result(&mut conn)
            .await;
        assert_eq!(versions_changed, Ok(false));
        assert_eq!(crates_changed, Ok(false));
    }
}
