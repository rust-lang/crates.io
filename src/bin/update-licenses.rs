//! Updates all of the licenses from the existing crates into each of their
//! already existing versions.

//
// Usage:
//      cargo run --bin update-licenses

extern crate cargo_registry;
extern crate postgres;

use std::io::prelude::*;

fn main() {
    let conn = cargo_registry::db::connect_now_old();
    {
        let tx = conn.transaction().unwrap();
        transfer(&tx);
        tx.set_commit();
        tx.finish().unwrap();
    }
}

fn transfer(tx: &postgres::transaction::Transaction) {
    let stmt = tx.prepare("SELECT id, name, license FROM crates").unwrap();
    let rows = stmt.query(&[]).unwrap();

    for row in rows.iter() {
        let id: i32 = row.get("id");
        let name: String = row.get("name");
        let license: Option<String> = row.get("license");

        if let Some(license) = license {
            println!(
                "Setting the license for all versions of {} to {}.",
                name,
                license
            );

            let num_updated = tx.execute(
                "UPDATE versions SET license = $1 WHERE crate_id = $2",
                &[&license, &id],
            ).unwrap();
            assert!(num_updated > 0);
        } else {
            println!(
                "Ignoring crate `{}` because it doesn't have a license.",
                name
            );
        }
    }

    get_confirm("Finish committing?");
}

fn get_confirm(msg: &str) {
    print!("{} [y/N]: ", msg);
    std::io::stdout().flush().unwrap();

    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();

    if !line.starts_with("y") {
        std::process::exit(0);
    }
}
