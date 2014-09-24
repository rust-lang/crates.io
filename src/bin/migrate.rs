extern crate "cargo-registry" as cargo_registry;
extern crate migrate;
extern crate postgres;
extern crate r2d2;

use std::os;
use migrate::Migration;
use postgres::{PostgresTransaction, PostgresResult};

fn main() {
    let db_config = r2d2::Config {
        pool_size: 1,
        helper_tasks: 1,
        test_on_check_out: false,
    };
    let database = cargo_registry::db::pool(env("DATABASE_URL").as_slice(),
                                            db_config);
    let conn = database.get().unwrap();
    let migrations = migrations();

    if os::args().as_slice().get(1).map(|s| s.as_slice()) == Some("rollback") {
        rollback(conn.transaction().unwrap(), migrations).unwrap();
    } else {
        apply(conn.transaction().unwrap(), migrations).unwrap();
    }

    fn env(s: &str) -> String {
        match os::getenv(s) {
            Some(s) => s,
            None => fail!("must have `{}` defined", s),
        }
    }
}

fn apply(tx: PostgresTransaction,
         migrations: Vec<Migration>) -> PostgresResult<()> {
    let mut mgr = try!(migrate::Manager::new(tx));
    for m in migrations.into_iter() {
        try!(mgr.apply(m));
    }
    mgr.set_commit();
    mgr.finish()
}

fn rollback(tx: PostgresTransaction,
            migrations: Vec<Migration>) -> PostgresResult<()> {
    let mut mgr = try!(migrate::Manager::new(tx));
    for m in migrations.into_iter().rev() {
        if mgr.contains(m.version()) {
            try!(mgr.rollback(m));
            break
        }
    }
    mgr.set_commit();
    mgr.finish()
}

fn migrations() -> Vec<Migration> {
    // Generate a new id via `date +"%Y%m%d%H%M%S"`
    vec![
        Migration::add_table(20140924113530, "users", "
            id              SERIAL PRIMARY KEY,
            email           VARCHAR NOT NULL UNIQUE,
            gh_access_token VARCHAR NOT NULL,
            api_token       VARCHAR NOT NULL
        "),
        Migration::add_table(20140924114003, "packages", "
            id              SERIAL PRIMARY KEY,
            name            VARCHAR NOT NULL UNIQUE,
            user_id         INTEGER NOT NULL
        "),
        Migration::add_table(20140924114059, "versions", "
            id              SERIAL PRIMARY KEY,
            package_id      INTEGER NOT NULL,
            num             VARCHAR NOT NULL UNIQUE
        "),
        Migration::run(20140924115329,
                       format!("ALTER TABLE versions ADD CONSTRAINT \
                                unique_num UNIQUE (package_id, num)"),
                       format!("ALTER TABLE versions DROP CONSTRAINT \
                                unique_num")),
        Migration::add_table(20140924120803, "version_dependencies", "
            version_id      INTEGER NOT NULL,
            depends_on_id   INTEGER NOT NULL
        "),
    ]
}
