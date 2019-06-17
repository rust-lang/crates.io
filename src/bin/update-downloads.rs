#![deny(warnings, clippy::all, rust_2018_idioms)]

#[macro_use]
extern crate diesel;

use cargo_registry::{
    db,
    models::VersionDownload,
    schema::{crates, metadata, version_downloads, versions},
    util::CargoResult,
};

use diesel::prelude::*;

fn main() -> CargoResult<()> {
    let conn = db::connect_now()?;
    update(&conn)?;
    Ok(())
}

fn update(conn: &PgConnection) -> QueryResult<()> {
    use crate::version_downloads::dsl::*;
    use diesel::dsl::now;
    use diesel::select;

    let rows = version_downloads
        .filter(processed.eq(false))
        .filter(downloads.ne(counted))
        .load(conn)?;
    collect(conn, &rows)?;

    // Anything older than 24 hours ago will be frozen and will not be queried
    // against again.
    diesel::update(version_downloads)
        .set(processed.eq(true))
        .filter(date.lt(diesel::dsl::date(now)))
        .filter(downloads.eq(counted))
        .filter(processed.eq(false))
        .execute(conn)?;

    no_arg_sql_function!(refresh_recent_crate_downloads, ());
    select(refresh_recent_crate_downloads).execute(conn)?;
    Ok(())
}

fn collect(conn: &PgConnection, rows: &[VersionDownload]) -> QueryResult<()> {
    use diesel::update;

    println!("updating {} versions", rows.len());

    for download in rows {
        let amt = download.downloads - download.counted;

        conn.transaction::<_, diesel::result::Error, _>(|| {
            // increment the number of counted downloads
            update(version_downloads::table.find(download.id()))
                .set(version_downloads::counted.eq(version_downloads::counted + amt))
                .execute(conn)?;

            // Update the total number of version downloads
            let crate_id = update(versions::table.find(download.version_id))
                .set(versions::downloads.eq(versions::downloads + amt))
                .returning(versions::crate_id)
                .get_result::<i32>(conn)?;

            // Update the total number of crate downloads
            update(crates::table.find(crate_id))
                .set(crates::downloads.eq(crates::downloads + amt))
                .execute(conn)?;

            // Now that everything else for this crate is done, update the global counter of total
            // downloads
            update(metadata::table)
                .set(metadata::total_downloads.eq(metadata::total_downloads + i64::from(amt)))
                .execute(conn)?;

            Ok(())
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use cargo_registry::{
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
        NewUser::new(2, "login", None, None, None, "access_token")
            .create_or_update(conn)
            .unwrap()
    }

    fn crate_and_version(conn: &PgConnection, user_id: i32) -> (Crate, Version) {
        let krate = NewCrate {
            name: "foo",
            ..Default::default()
        }
        .create_or_update(conn, None, user_id, None)
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
                version_downloads::date.eq(date(now - 1.day())),
                version_downloads::processed.eq(true),
            ))
            .execute(&conn)
            .unwrap();

        crate::update(&conn).unwrap();
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
        crate::update(&conn).unwrap();
        let version_downloads = versions::table
            .find(version.id)
            .select(versions::downloads)
            .first(&conn);
        assert_eq!(Ok(1), version_downloads);
    }

    #[test]
    fn set_processed_true() {
        use diesel::dsl::*;

        let conn = conn();
        let user = user(&conn);
        let (_, version) = crate_and_version(&conn, user.id);
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::downloads.eq(2),
                version_downloads::counted.eq(2),
                version_downloads::date.eq(date(now - 2.days())),
                version_downloads::processed.eq(false),
            ))
            .execute(&conn)
            .unwrap();
        crate::update(&conn).unwrap();
        let processed = version_downloads::table
            .filter(version_downloads::version_id.eq(version.id))
            .select(version_downloads::processed)
            .first(&conn);
        assert_eq!(Ok(true), processed);
    }

    #[test]
    fn dont_process_recent_row() {
        use diesel::dsl::*;
        let conn = conn();
        let user = user(&conn);
        let (_, version) = crate_and_version(&conn, user.id);
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::downloads.eq(2),
                version_downloads::counted.eq(2),
                version_downloads::date.eq(date(now)),
                version_downloads::processed.eq(false),
            ))
            .execute(&conn)
            .unwrap();
        crate::update(&conn).unwrap();
        let processed = version_downloads::table
            .filter(version_downloads::version_id.eq(version.id))
            .select(version_downloads::processed)
            .first(&conn);
        assert_eq!(Ok(false), processed);
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
                version_downloads::processed.eq(false),
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
        crate::update(&conn).unwrap();
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
        crate::update(&conn).unwrap();
        let version3 = versions::table
            .find(version.id)
            .first::<Version>(&conn)
            .unwrap();
        assert_eq!(version3.downloads, 2);
    }

    #[test]
    fn set_processed_no_set_updated_at() {
        use diesel::dsl::*;
        use diesel::update;

        let conn = conn();
        let user = user(&conn);
        let (_, version) = crate_and_version(&conn, user.id);
        update(versions::table)
            .set(versions::updated_at.eq(now - 2.days()))
            .execute(&conn)
            .unwrap();
        update(crates::table)
            .set(crates::updated_at.eq(now - 2.days()))
            .execute(&conn)
            .unwrap();
        insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version.id),
                version_downloads::downloads.eq(2),
                version_downloads::counted.eq(2),
                version_downloads::date.eq(date(now - 2.days())),
                version_downloads::processed.eq(false),
            ))
            .execute(&conn)
            .unwrap();

        crate::update(&conn).unwrap();
        let versions_changed = versions::table
            .select(versions::updated_at.ne(now - 2.days()))
            .get_result(&conn);
        let crates_changed = crates::table
            .select(crates::updated_at.ne(now - 2.days()))
            .get_result(&conn);
        assert_eq!(Ok(false), versions_changed);
        assert_eq!(Ok(false), crates_changed);
    }
}
