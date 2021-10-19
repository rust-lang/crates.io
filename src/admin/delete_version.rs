use crate::{
    admin::dialoguer,
    db,
    models::{Crate, Version},
    schema::versions,
};

use diesel::prelude::*;

#[derive(clap::Parser, Debug)]
#[clap(
    name = "delete-version",
    about = "Purge all references to a crate's version from the database.",
    after_help = "Please be super sure you want to do this before running this!"
)]
pub struct Opts {
    /// Name of the crate
    crate_name: String,
    /// Version number that should be deleted
    version: String,
}

pub fn run(opts: Opts) {
    let conn = db::connect_now().unwrap();
    conn.transaction::<_, diesel::result::Error, _>(|| {
        delete(opts, &conn);
        Ok(())
    })
    .unwrap()
}

fn delete(opts: Opts, conn: &PgConnection) {
    let krate: Crate = Crate::by_name(&opts.crate_name).first(conn).unwrap();
    let v: Version = Version::belonging_to(&krate)
        .filter(versions::num.eq(&opts.version))
        .first(conn)
        .unwrap();

    let prompt = format!(
        "Are you sure you want to delete {}#{} ({})?",
        opts.crate_name, opts.version, v.id
    );
    if !dialoguer::confirm(&prompt) {
        return;
    }

    println!("deleting version {} ({})", v.num, v.id);
    diesel::delete(versions::table.find(&v.id))
        .execute(conn)
        .unwrap();

    if !dialoguer::confirm("commit?") {
        panic!("aborting transaction");
    }
}
