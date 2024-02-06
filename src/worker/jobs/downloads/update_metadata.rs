use crate::models::VersionDownload;
use crate::schema::{crates, metadata, version_downloads, versions};
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct UpdateDownloads;

impl BackgroundJob for UpdateDownloads {
    const JOB_NAME: &'static str = "update_downloads";

    type Context = Arc<Environment>;

    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        spawn_blocking(move || {
            let mut conn = env.connection_pool.get()?;
            update(&mut conn)?;
            Ok(())
        })
        .await
    }
}

fn update(conn: &mut PgConnection) -> QueryResult<()> {
    use diesel::dsl::now;
    use diesel::select;

    let rows = version_downloads::table
        .filter(version_downloads::processed.eq(false))
        .filter(version_downloads::downloads.ne(version_downloads::counted))
        .load(conn)?;

    info!(rows = rows.len(), "Updating versions");
    collect(conn, &rows)?;
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

    sql_function!(fn refresh_recent_crate_downloads());
    select(refresh_recent_crate_downloads()).execute(conn)?;
    info!("Finished running refresh_recent_crate_downloads");

    Ok(())
}

fn collect(conn: &mut PgConnection, rows: &[VersionDownload]) -> QueryResult<()> {
    use diesel::update;

    for download in rows {
        let amt = download.downloads - download.counted;

        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            // Update the total number of version downloads
            let crate_id: i32 = update(versions::table.find(download.version_id))
                .set(versions::downloads.eq(versions::downloads + amt))
                .returning(versions::crate_id)
                .get_result(conn)?;

            // Update the total number of crate downloads
            update(crates::table.find(crate_id))
                .set(crates::downloads.eq(crates::downloads + amt))
                .execute(conn)?;

            // Update the global counter of total downloads
            update(metadata::table)
                .set(metadata::total_downloads.eq(metadata::total_downloads + i64::from(amt)))
                .execute(conn)?;

            // Record that these downloads have been propagated to the other tables.  This is done
            // last, immediately before the transaction is committed, to minimize lock contention
            // with counting new downloads.
            update(version_downloads::table.find(download.id()))
                .set(version_downloads::counted.eq(version_downloads::counted + amt))
                .execute(conn)?;

            Ok(())
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::email::Emails;
    use crate::models::{Crate, NewCrate, NewUser, NewVersion, User, Version};
    use crate::test_util::test_db_connection;
    use std::collections::BTreeMap;

    fn user(conn: &mut PgConnection) -> User {
        NewUser::new(2, "login", None, None, "access_token")
            .create_or_update(None, &Emails::new_in_memory(), conn)
            .unwrap()
    }

    fn crate_and_version(conn: &mut PgConnection, user_id: i32) -> (Crate, Version) {
        let krate = NewCrate {
            name: "foo",
            ..Default::default()
        }
        .create(conn, user_id)
        .unwrap();
        let version = NewVersion::new(
            krate.id,
            &semver::Version::parse("1.0.0").unwrap(),
            &BTreeMap::new(),
            None,
            0,
            user_id,
            "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            None,
            None,
        )
        .unwrap();
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
        let crate_downloads = crates::table
            .find(krate.id)
            .select(crates::downloads)
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
        assert_eq!(krate2.downloads, 2);
        assert_eq!(krate2.updated_at, krate_before.updated_at);
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
