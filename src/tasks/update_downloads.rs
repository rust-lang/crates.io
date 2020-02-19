use crate::{
    background_jobs::Environment,
    models::VersionDownload,
    schema::{crates, metadata, version_downloads, versions},
};

use diesel::prelude::*;
use swirl::PerformError;

#[swirl::background_job]
pub fn update_downloads(env: &Environment) -> Result<(), PerformError> {
    let conn = env.connection()?;
    update(&conn)?;
    Ok(())
}

fn update(conn: &PgConnection) -> QueryResult<()> {
    use self::version_downloads::dsl::*;
    use diesel::dsl::{now, IntervalDsl};
    use diesel::select;

    let rows = version_downloads
        .filter(downloads.ne(counted))
        .filter(date.ge(diesel::dsl::date(now - 1.week())))
        .load(conn)?;

    println!("Updating {} versions", rows.len());
    collect(conn, &rows)?;
    println!("Finished updating versions");

    no_arg_sql_function!(refresh_recent_crate_downloads, ());
    select(refresh_recent_crate_downloads).execute(conn)?;
    println!("Finished running refresh_recent_crate_downloads");

    Ok(())
}

fn collect(conn: &PgConnection, rows: &[VersionDownload]) -> QueryResult<()> {
    use diesel::update;

    for download in rows {
        let amt = download.downloads - download.counted;

        conn.transaction::<_, diesel::result::Error, _>(|| {
            // Update the total number of version downloads
            let crate_id = update(versions::table.find(download.version_id))
                .set(versions::downloads.eq(versions::downloads + amt))
                .returning(versions::crate_id)
                .get_result::<i32>(conn)?;

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
    use crate::{
        env,
        models::{Crate, NewCrate, NewUser, NewVersion, User, Version},
    };
    use std::collections::HashMap;

    fn conn() -> PgConnection {
        let conn = PgConnection::establish(&env("TEST_DATABASE_URL")).unwrap();
        conn.begin_test_transaction().unwrap();
        conn
    }

    fn user(conn: &PgConnection) -> User {
        NewUser::new(2, "login", None, None, "access_token")
            .create_or_update(None, conn)
            .unwrap()
    }

    fn crate_and_version(conn: &PgConnection, user_id: i32) -> (Crate, Version) {
        let krate = NewCrate {
            name: "foo",
            ..Default::default()
        }
        .create_or_update(conn, user_id, None)
        .unwrap();
        let version = NewVersion::new(
            krate.id,
            &semver::Version::parse("1.0.0").unwrap(),
            &HashMap::new(),
            None,
            None,
            0,
            user_id,
        )
        .unwrap();
        let version = version.save(conn, &[], "someone@example.com").unwrap();
        (krate, version)
    }

    #[test]
    fn increment() {
        use diesel::dsl::*;

        let conn = conn();
        let user = user(&conn);
        let (krate, version) = crate_and_version(&conn, user.id);
        insert_into(version_downloads::table)
            .values(version_downloads::version_id.eq(version.id))
            .execute(&conn)
            .unwrap();
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::date.eq(date(now - 8.days())),
            ))
            .execute(&conn)
            .unwrap();

        super::update(&conn).unwrap();
        let version_downloads = versions::table
            .find(version.id)
            .select(versions::downloads)
            .first(&conn);
        assert_eq!(Ok(1), version_downloads);
        let crate_downloads = crates::table
            .find(krate.id)
            .select(crates::downloads)
            .first(&conn);
        assert_eq!(Ok(1), crate_downloads);
        super::update(&conn).unwrap();
        let version_downloads = versions::table
            .find(version.id)
            .select(versions::downloads)
            .first(&conn);
        assert_eq!(Ok(1), version_downloads);
    }

    #[test]
    fn increment_a_little() {
        use diesel::dsl::*;
        use diesel::update;

        let conn = conn();
        let user = user(&conn);
        let (krate, version) = crate_and_version(&conn, user.id);
        update(versions::table)
            .set(versions::updated_at.eq(now - 2.hours()))
            .execute(&conn)
            .unwrap();
        update(crates::table)
            .set(crates::updated_at.eq(now - 2.hours()))
            .execute(&conn)
            .unwrap();
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::downloads.eq(2),
                version_downloads::counted.eq(1),
                version_downloads::date.eq(date(now)),
            ))
            .execute(&conn)
            .unwrap();
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::date.eq(date(now - 1.day())),
            ))
            .execute(&conn)
            .unwrap();

        let version_before = versions::table
            .find(version.id)
            .first::<Version>(&conn)
            .unwrap();
        let krate_before = Crate::all()
            .filter(crates::id.eq(krate.id))
            .first::<Crate>(&conn)
            .unwrap();
        super::update(&conn).unwrap();
        let version2 = versions::table
            .find(version.id)
            .first::<Version>(&conn)
            .unwrap();
        assert_eq!(version2.downloads, 2);
        assert_eq!(version2.updated_at, version_before.updated_at);
        let krate2 = Crate::all()
            .filter(crates::id.eq(krate.id))
            .first::<Crate>(&conn)
            .unwrap();
        assert_eq!(krate2.downloads, 2);
        assert_eq!(krate2.updated_at, krate_before.updated_at);
        super::update(&conn).unwrap();
        let version3 = versions::table
            .find(version.id)
            .first::<Version>(&conn)
            .unwrap();
        assert_eq!(version3.downloads, 2);
    }
}
