#![deny(warnings)]

extern crate cargo_registry;
extern crate chrono;
extern crate diesel;

use diesel::prelude::*;
use std::env;
use std::time::Duration;

use cargo_registry::models::VersionDownload;
use cargo_registry::schema::*;

static LIMIT: i64 = 1000;

fn main() {
    let daemon = env::args().nth(1).as_ref().map(|s| &s[..]) == Some("daemon");
    let sleep = env::args().nth(2).map(|s| s.parse().unwrap());
    loop {
        let conn = cargo_registry::db::connect_now().unwrap();
        update(&conn).unwrap();
        drop(conn);
        if daemon {
            std::thread::sleep(Duration::new(sleep.unwrap(), 0));
        } else {
            break;
        }
    }
}

fn update(conn: &PgConnection) -> QueryResult<()> {
    use version_downloads::dsl::*;
    let mut max = Some(0);
    while let Some(m) = max {
        conn.transaction::<_, diesel::result::Error, _>(|| {
            let rows = version_downloads
                .filter(processed.eq(false))
                .filter(id.gt(m))
                .order(id)
                .limit(LIMIT)
                .load(conn)?;
            collect(conn, &rows)?;
            max = rows.last().map(|d| d.id);
            Ok(())
        })?;
    }
    Ok(())
}

fn collect(conn: &PgConnection, rows: &[VersionDownload]) -> QueryResult<()> {
    use diesel::{insert_into, update};

    // Anything older than 24 hours ago will be frozen and will not be queried
    // against again.
    let now = chrono::Utc::now();
    let cutoff = now.naive_utc().date() - chrono::Duration::days(1);

    println!(
        "updating {} versions (cutoff {})",
        rows.len(),
        now.to_rfc2822()
    );

    let mut total = 0;
    for download in rows {
        let amt = download.downloads - download.counted;
        total += i64::from(amt);

        // Flag this row as having been processed if we're passed the cutoff,
        // and unconditionally increment the number of counted downloads.
        update(version_downloads::table.find(download.id))
            .set((
                version_downloads::processed.eq(download.date < cutoff),
                version_downloads::counted.eq(version_downloads::counted + amt),
            ))
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

        // Update the total number of crate downloads for today
        insert_into(crate_downloads::table)
            .values((
                crate_downloads::crate_id.eq(crate_id),
                crate_downloads::downloads.eq(amt),
                crate_downloads::date.eq(download.date),
            ))
            .on_conflict(crate_downloads::table.primary_key())
            .do_update()
            .set(crate_downloads::downloads.eq(crate_downloads::downloads + amt))
            .execute(conn)?;
    }

    // After everything else is done, update the global counter of total
    // downloads.
    update(metadata::table)
        .set(metadata::total_downloads.eq(metadata::total_downloads + total))
        .execute(conn)?;

    Ok(())
}

#[cfg(test)]
mod test {
    extern crate semver;

    use std::collections::HashMap;

    use diesel::insert_into;
    use super::*;
    use cargo_registry::env;
    use cargo_registry::models::{Crate, NewCrate, NewUser, NewVersion, User, Version};

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
        }.create_or_update(&conn, None, user_id)
            .unwrap();
        let version = NewVersion::new(
            krate.id,
            &semver::Version::parse("1.0.0").unwrap(),
            &HashMap::new(),
            None,
            None,
        ).unwrap();
        let version = version.save(&conn, &[]).unwrap();
        (krate, version)
    }

    macro_rules! crate_downloads {
        ($conn: expr, $id: expr, $expected: expr) => {
            let dl = crate_downloads::table
                .filter(crate_downloads::crate_id.eq($id))
                .select(crate_downloads::downloads)
                .first($conn);
            assert_eq!(Ok($expected as i32), dl);
        }
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

        ::update(&conn).unwrap();
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
        crate_downloads!(&conn, krate.id, 1);
        ::update(&conn).unwrap();
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
        ::update(&conn).unwrap();
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
                version_downloads::date.eq(date(now - 2.hours())),
                version_downloads::processed.eq(false),
            ))
            .execute(&conn)
            .unwrap();
        ::update(&conn).unwrap();
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
        ::update(&conn).unwrap();
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
        crate_downloads!(&conn, krate.id, 1);
        ::update(&conn).unwrap();
        let version3 = versions::table
            .find(version.id)
            .first::<Version>(&conn)
            .unwrap();
        assert_eq!(version3.downloads, 2);
    }

    #[test]
    fn set_processed_no_set_updated_at() {
        use diesel::update;
        use diesel::dsl::*;

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

        ::update(&conn).unwrap();
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
