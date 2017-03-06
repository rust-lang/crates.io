// Update the max_version for all crates.
//
// Usage:
//      cargo run --bin update-max-versions

#![deny(warnings)]

extern crate cargo_registry;
extern crate postgres;
extern crate semver;

fn main() {
    let conn = cargo_registry::db::connect_now();
    {
        let tx = conn.transaction().unwrap();
        update(&tx);
        tx.set_commit();
        tx.finish().unwrap();
    }
}

fn update(tx: &postgres::transaction::Transaction) {
    let crate_ids = tx.query("SELECT id FROM crates", &[]).unwrap();
    for crate_id in crate_ids.iter() {
        let crate_id: i32 = crate_id.get("id");
        let new_max = tx.query("SELECT num FROM versions WHERE crate_id = $1 AND yanked = FALSE",
                                &[&crate_id]).unwrap()
            .iter()
            .map(|r| r.get::<&str, String>("num"))
            .filter_map(|v| semver::Version::parse(&v).ok())
            .max();
        tx.execute("UPDATE crates SET max_version = $1 WHERE id = $2",
                     &[&new_max.map(|v| v.to_string()), &crate_id]).unwrap();
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use postgres;
    use semver;

    use cargo_registry::{Version, Crate, User, Model, env};

    fn conn() -> postgres::Connection {
        postgres::Connection::connect(&env("TEST_DATABASE_URL")[..],
                                      postgres::TlsMode::None).unwrap()
    }

    fn user(conn: &postgres::transaction::Transaction) -> User{
        User::find_or_insert(conn, 2, "login", None, None, None,
                             "access_token", "api_token").unwrap()
    }

    #[test]
    fn max_to_null() {
        let conn = conn();
        let tx = conn.transaction().unwrap();
        let user = user(&tx);
        let krate = Crate::find_or_insert(&tx, "foo", user.id, &None, &None,
                                          &None, &None, &None, &None,
                                          &None, None).unwrap();
        let v1 = semver::Version::parse("1.0.0").unwrap();
        let version = Version::insert(&tx, krate.id, &v1, &HashMap::new(), &[]).unwrap();
        version.yank(&conn, true).unwrap();
        ::update(&tx);
        assert_eq!(Crate::find(&tx, krate.id).unwrap().max_version, None);
    }

    #[test]
    fn max_to_same() {
        let conn = conn();
        let tx = conn.transaction().unwrap();
        let user = user(&tx);
        let krate = Crate::find_or_insert(&tx, "foo", user.id, &None, &None,
                                          &None, &None, &None, &None,
                                          &None, None).unwrap();
        let v1 = semver::Version::parse("1.0.0").unwrap();
        Version::insert(&tx, krate.id, &v1, &HashMap::new(), &[]).unwrap();
        ::update(&tx);
        assert_eq!(Crate::find(&tx, krate.id).unwrap().max_version, Some(v1));
    }

    #[test]
    fn multiple_crates() {
        let conn = conn();
        let tx = conn.transaction().unwrap();
        let user = user(&tx);
        let krate1 = Crate::find_or_insert(&tx, "foo1", user.id, &None, &None,
                                           &None, &None, &None, &None,
                                           &None, None).unwrap();
        let krate2 = Crate::find_or_insert(&tx, "foo2", user.id, &None, &None,
                                           &None, &None, &None, &None,
                                           &None, None).unwrap();
        let v1 = semver::Version::parse("1.0.0").unwrap();
        let krate1_ver = Version::insert(&tx, krate1.id, &v1, &HashMap::new(),
                                         &[]).unwrap();
        Version::insert(&tx, krate2.id, &v1, &HashMap::new(), &[]).unwrap();
        krate1_ver.yank(&conn, true).unwrap();
        ::update(&tx);
        assert_eq!(Crate::find(&tx, krate1.id).unwrap().max_version, None);
        assert_eq!(Crate::find(&tx, krate2.id).unwrap().max_version, Some(v1));
    }
}
