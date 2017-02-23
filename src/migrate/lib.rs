#![deny(warnings)]

extern crate postgres;

use std::collections::HashSet;

use postgres::transaction::Transaction;
use postgres::Result as PgResult;

struct A<'a, 'b: 'a> {
    t: &'a Transaction<'b>,
}

type Step = Box<FnMut(A) -> PgResult<()> + 'static>;

pub struct Migration {
    version: i64,
    up: Step,
    down: Step,
}

pub struct Manager<'a> {
    tx: Transaction<'a>,
    versions: HashSet<i64>,
}

impl Migration {
    fn mk(version: i64, up: Step, down: Step) -> Migration {
        Migration { version: version, up: up, down: down }
    }

    pub fn new<F1, F2>(version: i64, mut up: F1, mut down: F2) -> Migration
                       where F1: FnMut(&Transaction) -> PgResult<()> + 'static,
                             F2: FnMut(&Transaction) -> PgResult<()> + 'static
    {
        Migration::mk(version,
                      Box::new(move |a| up(a.t)),
                      Box::new(move |a| down(a.t)))
    }

    pub fn run(version: i64, up: &str, down: &str) -> Migration {
        Migration::mk(version,
                      run(up.to_string()),
                      run(down.to_string()))
    }

    pub fn add_table(version: i64, table: &str, rest: &str) -> Migration {
        let add_sql = format!("CREATE TABLE {} ({})", table, rest);
        let rm_sql = format!("DROP TABLE {}", table);
        Migration::mk(version, run(add_sql), run(rm_sql))
    }

    pub fn add_column(version: i64, table: &str, column: &str,
                      type_and_constraints: &str) -> Migration {
        let add_sql = format!("ALTER TABLE {} ADD COLUMN {} {}",
                              table, column, type_and_constraints);
        let rm_sql = format!("ALTER TABLE {} DROP COLUMN {}", table, column);
        Migration::mk(version, run(add_sql), run(rm_sql))
    }

    pub fn version(&self) -> i64 { self.version }
}

fn run(sql: String) -> Step {
    Box::new(move |a: A| {
        let tx = a.t;
        tx.execute(&sql, &[]).map(|_| ()).map_err(|e| {
            println!("failed to run `{}`", sql);
            e
        })
    })
}

impl<'a> Manager<'a> {
    pub fn new(tx: Transaction) -> PgResult<Manager> {
        let mut mgr = Manager { tx: tx, versions: HashSet::new() };
        mgr.load()?;
        Ok(mgr)
    }

    fn load(&mut self) -> PgResult<()> {
        self.tx.execute("CREATE TABLE IF NOT EXISTS schema_migrations (
            id              SERIAL PRIMARY KEY,
            version         INT8 NOT NULL UNIQUE
        )", &[])?;

        let stmt = self.tx.prepare("SELECT version FROM \
                                         schema_migrations")?;
        for row in stmt.query(&[])?.iter() {
            assert!(self.versions.insert(row.get("version")));
        }
        Ok(())
    }

    pub fn contains(&self, version: i64) -> bool {
        self.versions.contains(&version)
    }

    pub fn apply(&mut self, mut migration: Migration) -> PgResult<()> {
        if !self.versions.insert(migration.version) { return Ok(()) }
        println!("applying {}", migration.version);
        (migration.up)(A { t: &self.tx })?;
        let stmt = self.tx.prepare("INSERT into schema_migrations
                                         (version) VALUES ($1)")?;
        stmt.execute(&[&migration.version])?;
        Ok(())
    }

    pub fn rollback(&mut self, mut migration: Migration) -> PgResult<()> {
        if !self.versions.remove(&migration.version) { return Ok(()) }
        println!("rollback {}", migration.version);
        (migration.down)(A { t: &self.tx })?;
        let stmt = self.tx.prepare("DELETE FROM schema_migrations
                                         WHERE version = $1")?;
        stmt.execute(&[&migration.version])?;
        Ok(())
    }

    pub fn set_commit(&mut self) { self.tx.set_commit() }

    pub fn finish(self) -> PgResult<()> { self.tx.finish() }
}

#[cfg(test)]
mod tests {
    use std::os;
    use postgres::{PostgresConnection, NoSsl};
    use super::{Manager, Migration};

    fn conn() -> PostgresConnection {
        let url = os::getenv("MIGRATE_TEST_DATABASE_URL").unwrap();
        PostgresConnection::connect(&url, &NoSsl).unwrap()
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
            }, |_| panic!())).unwrap();
            mgr.set_commit();
        }
        assert!(called);
        called = false;
        {
            let mut mgr = Manager::new(c.transaction().unwrap()).unwrap();
            mgr.apply(Migration::new(1, |_| {
                called = true; Ok(())
            }, |_| panic!())).unwrap();
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
            mgr.rollback(Migration::new(1, |_| panic!(), |_| {
                called = true; Ok(())
            })).unwrap();
            mgr.set_commit();
        }
        assert!(!called);
        {
            let mut mgr = Manager::new(c.transaction().unwrap()).unwrap();
            mgr.apply(Migration::new(1, |_| {
                called = true; Ok(())
            }, |_| panic!())).unwrap();
            mgr.set_commit();
        }
        assert!(called);
        called = false;
        {
            let mut mgr = Manager::new(c.transaction().unwrap()).unwrap();
            mgr.rollback(Migration::new(1, |_| panic!(), |_| {
                called = true; Ok(())
            })).unwrap();
            mgr.set_commit();
        }
        assert!(called);
    }
}
