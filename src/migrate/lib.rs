extern crate postgres;

use std::collections::HashSet;

use postgres::{PostgresTransaction, PostgresResult};

pub type Step = proc(&PostgresTransaction): 'static -> PostgresResult<()>;

pub struct Migration {
    version: i64,
    up: Step,
    down: Step,
}

pub struct Manager<'a> {
    tx: PostgresTransaction<'a>,
    versions: HashSet<i64>,
}

impl Migration {
    pub fn new(version: i64, up: Step, down: Step) -> Migration {
        Migration {
            version: version,
            up: up,
            down: down,
        }
    }

    pub fn run<T: Str>(version: i64, up: T, down: T) -> Migration {
        Migration::new(version, run(up.as_slice().to_string()),
                       run(down.as_slice().to_string()))
    }

    pub fn add_table(version: i64, table: &str, rest: &str) -> Migration {
        let add_sql = format!("CREATE TABLE {} ({})", table, rest);
        let rm_sql = format!("DROP TABLE {}", table);
        Migration::new(version, run(add_sql), run(rm_sql))
    }

    pub fn add_column(version: i64, table: &str, column: &str,
                      type_and_constraints: &str) -> Migration {
        let add_sql = format!("ALTER TABLE {} ADD COLUMN {} {}",
                              table, column, type_and_constraints);
        let rm_sql = format!("ALTER TABLE {} DROP COLUMN {}", table, column);
        Migration::new(version, run(add_sql), run(rm_sql))
    }

    pub fn version(&self) -> i64 { self.version }
}

fn run(sql: String) -> Step {
    proc(tx) {
        tx.execute(sql.as_slice(), []).map(|_| ()).map_err(|e| {
            println!("failed to run `{}`", sql);
            e
        })
    }
}

impl<'a> Manager<'a> {
    pub fn new(tx: PostgresTransaction) -> PostgresResult<Manager> {
        let mut mgr = Manager { tx: tx, versions: HashSet::new() };
        try!(mgr.load());
        Ok(mgr)
    }

    fn load(&mut self) -> PostgresResult<()> {
        try!(self.tx.execute("CREATE TABLE IF NOT EXISTS schema_migrations (
            id              SERIAL PRIMARY KEY,
            version         INT8 NOT NULL UNIQUE
        )", []));

        let stmt = try!(self.tx.prepare("SELECT version FROM \
                                         schema_migrations"));
        for row in try!(stmt.query([])) {
            assert!(self.versions.insert(row.get("version")));
        }
        Ok(())
    }

    pub fn contains(&self, version: i64) -> bool {
        self.versions.contains(&version)
    }

    pub fn apply(&mut self, migration: Migration) -> PostgresResult<()> {
        if !self.versions.insert(migration.version) { return Ok(()) }
        println!("applying {}", migration.version);
        try!((migration.up)(&self.tx));
        let stmt = try!(self.tx.prepare("INSERT into schema_migrations
                                         (version) VALUES ($1)"));
        try!(stmt.execute(&[&migration.version]));
        Ok(())
    }

    pub fn rollback(&mut self, migration: Migration) -> PostgresResult<()> {
        if !self.versions.remove(&migration.version) { return Ok(()) }
        println!("rollback {}", migration.version);
        try!((migration.down)(&self.tx));
        let stmt = try!(self.tx.prepare("DELETE FROM schema_migrations
                                         WHERE version = $1"));
        try!(stmt.execute(&[&migration.version]));
        Ok(())
    }

    pub fn set_commit(&mut self) { self.tx.set_commit() }

    pub fn finish(self) -> PostgresResult<()> { self.tx.finish() }
}

#[cfg(test)]
mod tests {
    use std::os;
    use postgres::{PostgresConnection, NoSsl};
    use super::{Manager, Migration};

    fn conn() -> PostgresConnection {
        let url = os::getenv("MIGRATE_TEST_DATABASE_URL").unwrap();
        PostgresConnection::connect(url.as_slice(), &NoSsl).unwrap()
    }

    #[test]
    fn no_reapply() {
        let c = conn();
        let c = c.transaction().unwrap();
        let mut called = false;
        {
            let mut mgr = Manager::new(c.transaction().unwrap()).unwrap();
            mgr.apply(Migration::new(1, |_| {
                called = true; Ok(())
            }, |_| fail!())).unwrap();
            mgr.set_commit();
        }
        assert!(called);
        called = false;
        {
            let mut mgr = Manager::new(c.transaction().unwrap()).unwrap();
            mgr.apply(Migration::new(1, |_| {
                called = true; Ok(())
            }, |_| fail!())).unwrap();
            mgr.set_commit();
        }
        assert!(!called);
    }

    #[test]
    fn rollback_then_apply() {
        let c = conn();
        let c = c.transaction().unwrap();
        let mut called = false;
        {
            let mut mgr = Manager::new(c.transaction().unwrap()).unwrap();
            mgr.rollback(Migration::new(1, |_| fail!(), |_| {
                called = true; Ok(())
            })).unwrap();
            mgr.set_commit();
        }
        assert!(!called);
        {
            let mut mgr = Manager::new(c.transaction().unwrap()).unwrap();
            mgr.apply(Migration::new(1, |_| {
                called = true; Ok(())
            }, |_| fail!())).unwrap();
            mgr.set_commit();
        }
        assert!(called);
        called = false;
        {
            let mut mgr = Manager::new(c.transaction().unwrap()).unwrap();
            mgr.rollback(Migration::new(1, |_| fail!(), |_| {
                called = true; Ok(())
            })).unwrap();
            mgr.set_commit();
        }
        assert!(called);
    }
}
