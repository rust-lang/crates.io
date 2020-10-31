#![warn(clippy::all, rust_2018_idioms)]

use cargo_registry::{
    db,
    models::{Crate, Version},
    schema::versions,
};
use std::io::{self, prelude::*};

use clap::Clap;
use diesel::prelude::*;

#[derive(Clap, Debug)]
#[clap(
    name = "delete-version",
    about = "Purge all references to a crate's version from the database.\n\nPlease be super sure you want to do this before running this."
)]
struct Opts {
    /// Name of the crate
    crate_name: String,
    /// Version number that should be deleted
    version: String,
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
    let v: Version = Version::belonging_to(&krate)
        .filter(versions::num.eq(&opts.version))
        .first(conn)
        .unwrap();
    print!(
        "Are you sure you want to delete {}#{} ({}) [y/N]: ",
        opts.crate_name, opts.version, v.id
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
