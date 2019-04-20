// Transfer all crates from one user to another.
//
// Usage:
//      cargo run --bin transfer-crates from-user to-user

#![deny(warnings, clippy::all, rust_2018_idioms)]

use cargo_registry::{
    db,
    models::{Crate, OwnerKind, User},
    schema::{crate_owners, crates, users},
};
use std::{
    env,
    io::{self, prelude::*},
    process::exit,
};

use diesel::prelude::*;

fn main() {
    let conn = db::connect_now().unwrap();
    conn.transaction::<_, diesel::result::Error, _>(|| {
        transfer(&conn);
        Ok(())
    })
    .unwrap()
}

fn transfer(conn: &PgConnection) {
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

    let from = users::table
        .filter(users::gh_login.eq(from))
        .first::<User>(conn)
        .unwrap();
    let to = users::table
        .filter(users::gh_login.eq(to))
        .first::<User>(conn)
        .unwrap();

    if from.gh_id != to.gh_id {
        println!("====================================================");
        println!("WARNING");
        println!();
        println!("this may not be the same github user, different github IDs");
        println!();
        println!("from: {:?}", from.gh_id);
        println!("to:   {:?}", to.gh_id);

        get_confirm("continue?");
    }

    println!(
        "Are you sure you want to transfer crates from {} to {}",
        from.gh_login, to.gh_login
    );
    get_confirm("continue");

    let crate_owners = crate_owners::table
        .filter(crate_owners::owner_id.eq(from.id))
        .filter(crate_owners::owner_kind.eq(OwnerKind::User as i32));
    let crates = Crate::all()
        .filter(crates::id.eq_any(crate_owners.select(crate_owners::crate_id)))
        .load::<Crate>(conn)
        .unwrap();

    for krate in crates {
        let owners = krate.owners(conn).unwrap();
        if owners.len() != 1 {
            println!("warning: not exactly one owner for {}", krate.name);
        }
    }

    diesel::update(crate_owners)
        .set(crate_owners::owner_id.eq(to.id))
        .execute(conn)
        .unwrap();

    get_confirm("commit?");
}

fn get_confirm(msg: &str) {
    print!("{} [y/N]: ", msg);
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    if !line.starts_with('y') {
        exit(0);
    }
}
