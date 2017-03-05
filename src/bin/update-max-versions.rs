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
