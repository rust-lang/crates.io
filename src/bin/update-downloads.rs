#![deny(warnings)]

extern crate cargo_registry;
extern crate chrono;
extern crate openssl;
extern crate postgres;
extern crate semver;
extern crate time;

use std::collections::HashMap;
use std::env;
use std::time::Duration;

use cargo_registry::{VersionDownload, Version, Model};

static LIMIT: i64 = 1000;

#[allow(dead_code)] // dead in tests
fn main() {
    let daemon = env::args().nth(1).as_ref().map(|s| &s[..]) == Some("daemon");
    let sleep = env::args().nth(2).map(|s| s.parse().unwrap());
    loop {
        let conn = cargo_registry::db::connect_now();
        update(&conn).unwrap();
        drop(conn);
        if daemon {
            std::thread::sleep(Duration::new(sleep.unwrap(), 0));
        } else {
            break;
        }
    }
}

fn update(conn: &postgres::GenericConnection) -> postgres::Result<()> {
    let mut max = 0;
    loop {
        // FIXME(rust-lang/rust#27401): weird declaration to make sure this
        // variable gets dropped.
        let tx;
        tx = conn.transaction()?;
        {
            let stmt = tx.prepare(
                "SELECT * FROM version_downloads \
                                        WHERE processed = FALSE AND id > $1
                                        ORDER BY id ASC
                                        LIMIT $2",
            )?;
            let mut rows = stmt.query(&[&max, &LIMIT])?;
            match collect(&tx, &mut rows)? {
                None => break,
                Some(m) => max = m,
            }
        }
        tx.set_commit();
        tx.finish()?;
    }
    Ok(())
}

fn collect(
    tx: &postgres::transaction::Transaction,
    rows: &mut postgres::rows::Rows,
) -> postgres::Result<Option<i32>> {
    // Anything older than 24 hours ago will be frozen and will not be queried
    // against again.
    let now = chrono::UTC::now();
    let cutoff = now.naive_utc().date() - chrono::Duration::days(1);

    let mut map = HashMap::new();
    for row in rows.iter() {
        let download: VersionDownload = Model::from_row(&row);
        assert!(map.insert(download.id, download).is_none());
    }
    println!(
        "updating {} versions (cutoff {})",
        map.len(),
        now.to_rfc2822()
    );
    if map.len() == 0 {
        return Ok(None);
    }

    let mut max = 0;
    let mut total = 0;
    for (id, download) in map.iter() {
        if *id > max {
            max = *id;
        }
        if download.date > cutoff && download.counted == download.downloads {
            continue;
        }
        let amt = download.downloads - download.counted;

        // Flag this row as having been processed if we're passed the cutoff,
        // and unconditionally increment the number of counted downloads.
        tx.execute(
            "UPDATE version_downloads
                         SET processed = $2, counted = counted + $3
                         WHERE id = $1",
            &[id, &(download.date < cutoff), &amt],
        )?;
        total += amt as i64;

        if amt == 0 {
            continue;
        }

        let crate_id = Version::find(tx, download.version_id).unwrap().crate_id;

        // Update the total number of version downloads
        tx.execute(
            "UPDATE versions
                         SET downloads = downloads + $1
                         WHERE id = $2",
            &[&amt, &download.version_id],
        )?;
        // Update the total number of crate downloads
        tx.execute(
            "UPDATE crates SET downloads = downloads + $1
                         WHERE id = $2",
            &[&amt, &crate_id],
        )?;

        // Update the total number of crate downloads for today
        let cnt = tx.execute(
            "UPDATE crate_downloads
                                   SET downloads = downloads + $2
                                   WHERE crate_id = $1 AND date = $3",
            &[&crate_id, &amt, &download.date],
        )?;
        if cnt == 0 {
            tx.execute(
                "INSERT INTO crate_downloads
                             (crate_id, downloads, date)
                             VALUES ($1, $2, $3)",
                &[&crate_id, &amt, &download.date],
            )?;
        }
    }

    // After everything else is done, update the global counter of total
    // downloads.
    tx.execute(
        "UPDATE metadata SET total_downloads = total_downloads + $1",
        &[&total],
    )?;

    Ok(Some(max))
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use time;
    use time::Duration;

    use postgres;
    use semver;

    use cargo_registry::{Version, Crate, User, Model, env};

    fn conn() -> postgres::Connection {
        postgres::Connection::connect(&env("TEST_DATABASE_URL")[..], postgres::TlsMode::None).unwrap()
    }

    fn user(conn: &postgres::transaction::Transaction) -> User {
        User::find_or_insert(conn, 2, "login", None, None, None, "access_token").unwrap()
    }

    fn crate_downloads(tx: &postgres::transaction::Transaction, id: i32, expected: usize) {
        let stmt = tx.prepare(
            "SELECT * FROM crate_downloads
                               WHERE crate_id = $1",
        ).unwrap();
        let dl: i32 = stmt.query(&[&id]).unwrap().iter().next().unwrap().get(
            "downloads",
        );
        assert_eq!(dl, expected as i32);
    }

    #[test]
    fn increment() {
        let conn = conn();
        let tx = conn.transaction().unwrap();
        let user = user(&tx);
        let krate = Crate::find_or_insert(
            &tx,
            "foo",
            user.id,
            &None,
            &None,
            &None,
            &None,
            &None,
            &None,
            &None,
            None,
        ).unwrap();
        let version = Version::insert(
            &tx,
            krate.id,
            &semver::Version::parse("1.0.0").unwrap(),
            &HashMap::new(),
            &[],
        ).unwrap();
        tx.execute(
            "INSERT INTO version_downloads \
                    (version_id)
                    VALUES ($1)",
            &[&version.id],
        ).unwrap();
        tx.execute(
            "INSERT INTO version_downloads \
                    (version_id, date, processed)
                    VALUES ($1, current_date - interval '1 day', true)",
            &[&version.id],
        ).unwrap();
        ::update(&tx).unwrap();
        assert_eq!(Version::find(&tx, version.id).unwrap().downloads, 1);
        assert_eq!(Crate::find(&tx, krate.id).unwrap().downloads, 1);
        crate_downloads(&tx, krate.id, 1);
        ::update(&tx).unwrap();
        assert_eq!(Version::find(&tx, version.id).unwrap().downloads, 1);
    }

    #[test]
    fn set_processed_true() {
        let conn = conn();
        let tx = conn.transaction().unwrap();
        let user = user(&tx);
        let krate = Crate::find_or_insert(
            &tx,
            "foo",
            user.id,
            &None,
            &None,
            &None,
            &None,
            &None,
            &None,
            &None,
            None,
        ).unwrap();
        let version = Version::insert(
            &tx,
            krate.id,
            &semver::Version::parse("1.0.0").unwrap(),
            &HashMap::new(),
            &[],
        ).unwrap();
        tx.execute(
            "INSERT INTO version_downloads \
                    (version_id, downloads, counted, date, processed)
                    VALUES ($1, 2, 2, current_date - interval '2 days', false)",
            &[&version.id],
        ).unwrap();
        ::update(&tx).unwrap();
        let stmt = tx.prepare(
            "SELECT processed FROM version_downloads
                               WHERE version_id = $1",
        ).unwrap();
        let processed: bool = stmt.query(&[&version.id])
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .get("processed");
        assert!(processed);
    }

    #[test]
    fn dont_process_recent_row() {
        let conn = conn();
        let tx = conn.transaction().unwrap();
        let user = user(&tx);
        let krate = Crate::find_or_insert(
            &tx,
            "foo",
            user.id,
            &None,
            &None,
            &None,
            &None,
            &None,
            &None,
            &None,
            None,
        ).unwrap();
        let version = Version::insert(
            &tx,
            krate.id,
            &semver::Version::parse("1.0.0").unwrap(),
            &HashMap::new(),
            &[],
        ).unwrap();
        let time = time::now_utc().to_timespec() - Duration::hours(2);
        tx.execute(
            "INSERT INTO version_downloads \
                    (version_id, downloads, counted, date, processed)
                    VALUES ($1, 2, 2, date($2), false)",
            &[&version.id, &time],
        ).unwrap();
        ::update(&tx).unwrap();
        let stmt = tx.prepare(
            "SELECT processed FROM version_downloads
                               WHERE version_id = $1",
        ).unwrap();
        let processed: bool = stmt.query(&[&version.id])
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .get("processed");
        assert!(!processed);
    }

    #[test]
    fn increment_a_little() {
        let conn = conn();
        let tx = conn.transaction().unwrap();
        let user = user(&tx);
        let krate = Crate::find_or_insert(
            &tx,
            "foo",
            user.id,
            &None,
            &None,
            &None,
            &None,
            &None,
            &None,
            &None,
            None,
        ).unwrap();
        let version = Version::insert(
            &tx,
            krate.id,
            &semver::Version::parse("1.0.0").unwrap(),
            &HashMap::new(),
            &[],
        ).unwrap();
        tx.execute(
            "UPDATE versions
                       SET updated_at = current_date - interval '2 hours'",
            &[],
        ).unwrap();
        tx.execute(
            "UPDATE crates
                       SET updated_at = current_date - interval '2 hours'",
            &[],
        ).unwrap();
        tx.execute(
            "INSERT INTO version_downloads \
                    (version_id, downloads, counted, date, processed)
                    VALUES ($1, 2, 1, current_date, false)",
            &[&version.id],
        ).unwrap();
        tx.execute(
            "INSERT INTO version_downloads \
                    (version_id, date)
                    VALUES ($1, current_date - interval '1 day')",
            &[&version.id],
        ).unwrap();

        let version_before = Version::find(&tx, version.id).unwrap();
        let krate_before = Crate::find(&tx, krate.id).unwrap();
        ::update(&tx).unwrap();
        let version2 = Version::find(&tx, version.id).unwrap();
        assert_eq!(version2.downloads, 2);
        assert_eq!(version2.updated_at, version_before.updated_at);
        let krate2 = Crate::find(&tx, krate.id).unwrap();
        assert_eq!(krate2.downloads, 2);
        assert_eq!(krate2.updated_at, krate_before.updated_at);
        crate_downloads(&tx, krate.id, 1);
        ::update(&tx).unwrap();
        assert_eq!(Version::find(&tx, version.id).unwrap().downloads, 2);
    }

    #[test]
    fn set_processed_no_set_updated_at() {
        let conn = conn();
        let tx = conn.transaction().unwrap();
        let user = user(&tx);
        let krate = Crate::find_or_insert(
            &tx,
            "foo",
            user.id,
            &None,
            &None,
            &None,
            &None,
            &None,
            &None,
            &None,
            None,
        ).unwrap();
        let version = Version::insert(
            &tx,
            krate.id,
            &semver::Version::parse("1.0.0").unwrap(),
            &HashMap::new(),
            &[],
        ).unwrap();
        tx.execute(
            "UPDATE versions
                       SET updated_at = current_date - interval '2 days'",
            &[],
        ).unwrap();
        tx.execute(
            "UPDATE crates
                       SET updated_at = current_date - interval '2 days'",
            &[],
        ).unwrap();
        tx.execute(
            "INSERT INTO version_downloads \
                    (version_id, downloads, counted, date, processed)
                    VALUES ($1, 2, 2, current_date - interval '2 days', false)",
            &[&version.id],
        ).unwrap();

        let version_before = Version::find(&tx, version.id).unwrap();
        let krate_before = Crate::find(&tx, krate.id).unwrap();
        ::update(&tx).unwrap();
        let version2 = Version::find(&tx, version.id).unwrap();
        assert_eq!(version2.updated_at, version_before.updated_at);
        let krate2 = Crate::find(&tx, krate.id).unwrap();
        assert_eq!(krate2.updated_at, krate_before.updated_at);
    }
}
