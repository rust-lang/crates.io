// Sync available crate categories from `src/categories.txt`.
// Only needs to be run for new databases or when `src/categories.txt`
// has been updated.
//
// Usage:
//      cargo run --bin sync-categories

#![deny(warnings)]

extern crate cargo_registry;
extern crate postgres;

use cargo_registry::env;

fn main() {
    let conn = postgres::Connection::connect(&env("DATABASE_URL")[..],
                                             postgres::TlsMode::None).unwrap();
    let tx = conn.transaction().unwrap();
    sync(&tx).unwrap();
    tx.set_commit();
    tx.finish().unwrap();
}

fn sync(tx: &postgres::transaction::Transaction) -> postgres::Result<()> {
    let categories = include_str!("../categories.txt");
    let categories: Vec<_> = categories.lines().collect();
    let insert = categories.iter()
                           .map(|c| format!("('{}')", c))
                           .collect::<Vec<_>>()
                           .join(",");
    let in_clause = categories.iter()
                              .map(|c| format!("'{}'", c))
                              .collect::<Vec<_>>()
                              .join(",");

    try!(tx.batch_execute(
        &format!(" \
            INSERT INTO categories (category) \
            VALUES {} \
            ON CONFLICT (category) DO NOTHING; \
            DELETE FROM categories \
            WHERE category NOT IN ({});",
            insert,
            in_clause
        )[..]
    ));
    Ok(())
}
