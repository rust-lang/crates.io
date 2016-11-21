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

    let slug_categories: Vec<_> = categories.lines().map(|c| {
        let mut parts = c.split(' ');
        let slug = parts.next().expect("No slug found!");
        let name = parts.collect::<Vec<_>>().join(" ");
        (slug, name)
    }).collect();

    let insert = slug_categories.iter().map(|&(ref slug, ref name)| {
        format!("(LOWER('{}'), '{}')", slug, name)
    }).collect::<Vec<_>>().join(",");

    let in_clause = slug_categories.iter().map(|&(slug, _)| {
        format!("LOWER('{}')", slug)
    }).collect::<Vec<_>>().join(",");

    try!(tx.batch_execute(
        &format!(" \
            INSERT INTO categories (slug, category) \
            VALUES {} \
            ON CONFLICT (slug) DO UPDATE SET category = EXCLUDED.category; \
            DELETE FROM categories \
            WHERE slug NOT IN ({});",
            insert,
            in_clause
        )[..]
    ));
    Ok(())
}
