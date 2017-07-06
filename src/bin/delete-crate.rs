// Purge all references to a crate from the database.
//
// Please be super sure you want to do this before running this.
//
// Usage:
//      cargo run --bin delete-crate crate-name

#![deny(warnings)]

extern crate cargo_registry;
extern crate postgres;
extern crate time;

use std::env;
use std::io;
use std::io::prelude::*;

use cargo_registry::Crate;

#[allow(dead_code)]
fn main() {
    let conn = cargo_registry::db::connect_now_old();
    {
        let tx = conn.transaction().unwrap();
        delete(&tx);
        tx.set_commit();
        tx.finish().unwrap();
    }
}

fn delete(tx: &postgres::transaction::Transaction) {
    let name = match env::args().nth(1) {
        None => {
            println!("needs a crate-name argument");
            return;
        }
        Some(s) => s,
    };

    let krate = Crate::find_by_name(tx, &name).unwrap();
    print!(
        "Are you sure you want to delete {} ({}) [y/N]: ",
        name,
        krate.id
    );
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    if !line.starts_with("y") {
        return;
    }

    let versions = krate.versions(tx).unwrap();

    for v in versions.iter() {
        println!("deleting version {} ({})", v.num, v.id);
        let n = tx.execute(
            "DELETE FROM version_downloads WHERE version_id = $1",
            &[&v.id],
        ).unwrap();
        println!("  {} download records deleted", n);
        let n = tx.execute(
            "DELETE FROM version_authors WHERE version_id = $1",
            &[&v.id],
        ).unwrap();
        println!("  {} author records deleted", n);
        let n = tx.execute("DELETE FROM dependencies WHERE version_id = $1", &[&v.id])
            .unwrap();
        println!("  {} dependencies deleted", n);
        tx.execute("DELETE FROM versions WHERE id = $1", &[&v.id])
            .unwrap();
    }

    println!("deleting follows");
    let n = tx.execute("DELETE FROM follows WHERE crate_id = $1", &[&krate.id])
        .unwrap();
    println!("  {} deleted", n);

    println!("deleting crate download records");
    let n = tx.execute(
        "DELETE FROM crate_downloads WHERE crate_id = $1",
        &[&krate.id],
    ).unwrap();
    println!("  {} deleted", n);

    println!("deleting crate owners");
    let n = tx.execute("DELETE FROM crate_owners WHERE crate_id = $1", &[&krate.id])
        .unwrap();
    println!("  {} deleted", n);

    println!("disabling reserved crate name trigger");
    let _ = tx.execute(
        "ALTER TABLE crates DISABLE TRIGGER trigger_ensure_crate_name_not_reserved;",
        &[],
    ).unwrap();

    println!("deleting crate keyword connections");
    let n = tx.execute(
        "DELETE FROM crates_keywords WHERE crate_id = $1",
        &[&krate.id],
    ).unwrap();
    println!("  {} deleted", n);

    println!("deleting crate category connections");
    let n = tx.execute(
        "DELETE FROM crates_categories WHERE crate_id = $1",
        &[&krate.id],
    ).unwrap();
    println!("  {} deleted", n);

    println!("enabling reserved crate name trigger");
    let _ = tx.execute(
        "ALTER TABLE crates ENABLE TRIGGER trigger_ensure_crate_name_not_reserved;",
        &[],
    ).unwrap();

    println!("deleting crate badges");
    let n = tx.execute("DELETE FROM badges WHERE crate_id = $1", &[&krate.id])
        .unwrap();
    println!("  {} deleted", n);

    println!("deleting the crate");
    let n = tx.execute("DELETE FROM crates WHERE id = $1", &[&krate.id])
        .unwrap();
    println!("  {} deleted", n);

    print!("commit? [y/N]: ");
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    if !line.starts_with("y") {
        panic!("aborting transaction");
    }
}
