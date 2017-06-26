// Transfer all crates from one user to another.
//
// Usage:
//      cargo run --bin transfer-crates from-user to-user

#![deny(warnings)]

extern crate cargo_registry;
extern crate postgres;
extern crate time;
extern crate semver;

use std::env;
use std::io;
use std::io::prelude::*;

use cargo_registry::{Crate, User};
use cargo_registry::owner::OwnerKind;
use cargo_registry::Model;

#[allow(dead_code)]
fn main() {
    let conn = cargo_registry::db::connect_now();
    {
        let tx = conn.transaction().unwrap();
        transfer(&tx);
        tx.set_commit();
        tx.finish().unwrap();
    }
}

fn transfer(tx: &postgres::transaction::Transaction) {
    let from = match env::args().nth(1) {
        None => {
            println!("needs a from-user argument");
            return;
        }
        Some(s) => s,
    };
    let to = match env::args().nth(2) {
        None => {
            println!("needs a to-user argument");
            return;
        }
        Some(s) => s,
    };

    let from = User::find_by_login(tx, &from).unwrap();
    let to = User::find_by_login(tx, &to).unwrap();

    if from.gh_id != to.gh_id {
        println!("====================================================");
        println!("WARNING");
        println!("");
        println!("this may not be the same github user, different github IDs");
        println!("");
        println!("from: {:?}", from.gh_id);
        println!("to:   {:?}", to.gh_id);

        get_confirm("continue?");
    }

    println!(
        "Are you sure you want to transfer crates from {} to {}",
        from.gh_login,
        to.gh_login
    );
    get_confirm("continue");


    let stmt = tx.prepare(
        "SELECT * FROM crate_owners
                                   WHERE owner_id = $1
                                     AND owner_kind = $2",
    ).unwrap();
    let rows = stmt.query(&[&from.id, &(OwnerKind::User as i32)]).unwrap();
    for row in rows.iter() {
        let owner_id: i32 = row.get("owner_id");
        let krate = Crate::find(tx, row.get("crate_id")).unwrap();
        println!("transferring {}", krate.name);
        let owners = krate.owners_old(tx).unwrap();
        if owners.len() != 1 {
            println!("warning: not exactly one owner for {}", krate.name);
        }
        let n = tx.execute(
            "UPDATE crate_owners SET owner_id = $1
                             WHERE owner_id = $2",
            &[&to.id, &owner_id],
        ).unwrap();
        assert_eq!(n, 1);
    }

    get_confirm("commit?");
}

fn get_confirm(msg: &str) {
    print!("{} [y/N]: ", msg);
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    if !line.starts_with("y") {
        std::process::exit(0);
    }
}
