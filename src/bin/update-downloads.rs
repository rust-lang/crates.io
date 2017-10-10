#![deny(warnings)]

extern crate cargo_registry;
extern crate chrono;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::pg::upsert::*;
use std::env;
use std::time::Duration;

use cargo_registry::VersionDownload;
use cargo_registry::schema::*;

static LIMIT: i64 = 1000;

#[derive(Insertable)]
#[table_name = "crate_downloads"]
struct CrateDownload {
    crate_id: i32,
    downloads: i32,
    date: NaiveDate,
}

#[allow(dead_code)] // dead in tests
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
    use diesel::{insert, update};

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
        total += amt as i64;

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
            .get_result(conn)?;

        // Update the total number of crate downloads
        update(crates::table.find(crate_id))
            .set(crates::downloads.eq(crates::downloads + amt))
            .execute(conn)?;

        // Update the total number of crate downloads for today
        let crate_download = CrateDownload {
            crate_id: crate_id,
            downloads: amt,
            date: download.date,
        };
        insert(&crate_download.on_conflict(
            (crate_downloads::crate_id, crate_downloads::date),
            do_update().set(crate_downloads::downloads.eq(crate_downloads::downloads + amt)),
        )).into(crate_downloads::table)
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

    use diesel::expression::dsl::sql;
    use diesel::types::Integer;
    use super::*;
    use cargo_registry::env;
    use cargo_registry::krate::{Crate, NewCrate};
    use cargo_registry::user::{NewUser, User};
    use cargo_registry::version::{NewVersion, Version};

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
        let conn = conn();
        let user = user(&conn);
        let (krate, version) = crate_and_version(&conn, user.id);
        // FIXME: Diesel 1.0 can do this:
        // insert((version_id.eq(version.id),))
        //     .into(version_downloads)
        //     .execute(&conn)
        //     .unwrap();
        // insert((
        //     version_id.eq(version.id),
        //     date.eq(now - 1.day()),
        //     processed.eq(true)
        //  )).into(version_downloads)
        //      .execute(&conn)
        //      .unwrap();
        sql::<Integer>(
            "INSERT INTO version_downloads \
                    (version_id)
                    VALUES ($1)",
        ).bind::<Integer, _>(version.id)
            .execute(&conn)
            .unwrap();
        sql::<Integer>(
            "INSERT INTO version_downloads \
                    (version_id, date, processed)
                    VALUES ($1, current_date - interval '1 day', true)",
        ).bind::<Integer, _>(version.id)
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
        let conn = conn();
        let user = user(&conn);
        let (_, version) = crate_and_version(&conn, user.id);
        sql::<Integer>(
            "INSERT INTO version_downloads \
                    (version_id, downloads, counted, date, processed)
                    VALUES ($1, 2, 2, current_date - interval '2 days', false)",
        ).bind::<Integer, _>(version.id)
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
        let conn = conn();
        let user = user(&conn);
        let (_, version) = crate_and_version(&conn, user.id);
        sql::<Integer>(
            "INSERT INTO version_downloads \
                    (version_id, downloads, counted, date, processed)
                    VALUES ($1, 2, 2, DATE(NOW() - INTERVAL '2 hours'), false)",
        ).bind::<Integer, _>(version.id)
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
        use diesel::expression::dsl::*;
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
        sql::<Integer>(
            "INSERT INTO version_downloads \
                    (version_id, downloads, counted, date, processed)
                    VALUES ($1, 2, 1, current_date, false)",
        ).bind::<Integer, _>(version.id)
            .execute(&conn)
            .unwrap();
        sql::<Integer>(
            "INSERT INTO version_downloads \
                    (version_id, date)
                    VALUES ($1, current_date - interval '1 day')",
        ).bind::<Integer, _>(version.id)
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
        use diesel::expression::dsl::*;

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
        sql::<Integer>(
            "INSERT INTO version_downloads \
                    (version_id, downloads, counted, date, processed)
                    VALUES ($1, 2, 2, current_date - interval '2 days', false)",
        ).bind::<Integer, _>(version.id)
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
