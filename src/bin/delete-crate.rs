#![warn(clippy::all, rust_2018_idioms)]

use cargo_registry::{db, models::Crate, schema::crates};
use std::io::{self, prelude::*};

use clap::Clap;
use diesel::prelude::*;

#[derive(Clap, Debug)]
#[clap(
    name = "delete-crate",
    about = "Purge all references to a crate from the database.\n\nPlease be super sure you want to do this before running this."
)]
struct Opts {
    /// Name of the crate
    crate_name: String,
}

fn main() {
    let conn = db::connect_now().unwrap();
    conn.transaction::<_, diesel::result::Error, _>(|| {
        delete(&conn);
        Ok(())
    })
    .unwrap()
}

fn delete(conn: &PgConnection) {
    let opts: Opts = Opts::parse();

    let krate: Crate = Crate::by_name(&opts.crate_name).first(conn).unwrap();
    print!(
        "Are you sure you want to delete {} ({}) [y/N]: ",
        opts.crate_name, krate.id
    );
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    if !line.starts_with('y') {
        return;
    }

    println!("deleting the crate");
    let n = diesel::delete(crates::table.find(krate.id))
        .execute(conn)
        .unwrap();
    println!("  {} deleted", n);

    print!("commit? [y/N]: ");
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    if !line.starts_with('y') {
        panic!("aborting transaction");
    }
}
