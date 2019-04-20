// Purge all references to a crate's version from the database.
//
// Please be super sure you want to do this before running this.
//
// Usage:
//      cargo run --bin delete-version crate-name version-number

#![deny(warnings, clippy::all, rust_2018_idioms)]

use cargo_registry::{
    db,
    models::{Crate, Version},
    schema::versions,
};
use std::{
    env,
    io::{self, prelude::*},
};

use diesel::prelude::*;

fn main() {
    let conn = db::connect_now().unwrap();
    conn.transaction::<_, diesel::result::Error, _>(|| {
        delete(&conn);
        Ok(())
    })
    .unwrap()
}

fn delete(conn: &PgConnection) {
    let name = match env::args().nth(1) {
        None => {
            println!("needs a crate-name argument");
            return;
        }
        Some(s) => s,
    };
    let version = match env::args().nth(2) {
        None => {
            println!("needs a version argument");
            return;
        }
        Some(s) => s,
    };

    let krate = Crate::by_name(&name).first::<Crate>(conn).unwrap();
    let v = Version::belonging_to(&krate)
        .filter(versions::num.eq(&version))
        .first::<Version>(conn)
        .unwrap();
    print!(
        "Are you sure you want to delete {}#{} ({}) [y/N]: ",
        name, version, v.id
    );
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    if !line.starts_with('y') {
        return;
    }

    println!("deleting version {} ({})", v.num, v.id);
    diesel::delete(versions::table.find(&v.id))
        .execute(conn)
        .unwrap();

    print!("commit? [y/N]: ");
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    if !line.starts_with('y') {
        panic!("aborting transaction");
    }
}
