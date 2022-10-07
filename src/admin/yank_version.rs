use crate::{
    admin::dialoguer,
    db,
    models::{Crate, Version},
    schema::versions,
};

use diesel::prelude::*;
use swirl::Job;

#[derive(clap::Parser, Debug)]
#[command(
    name = "yank-version",
    about = "Yank a crate from the database and index."
)]
pub struct Opts {
    /// Name of the crate
    crate_name: String,
    /// Version number that should be deleted
    version: String,
    /// Don't ask for confirmation: yes, we are sure. Best for scripting.
    #[arg(short, long)]
    yes: bool,
}

pub fn run(opts: Opts) {
    let conn = db::oneoff_connection().unwrap();
    conn.transaction::<_, diesel::result::Error, _>(|| {
        yank(opts, &conn);
        Ok(())
    })
    .unwrap()
}

fn yank(opts: Opts, conn: &PgConnection) {
    let Opts {
        crate_name,
        version,
        yes,
    } = opts;
    let krate: Crate = Crate::by_name(&crate_name).first(conn).unwrap();
    let v: Version = Version::belonging_to(&krate)
        .filter(versions::num.eq(&version))
        .first(conn)
        .unwrap();

    if v.yanked {
        println!("Version {version} of crate {crate_name} is already yanked");
        return;
    }

    if !yes {
        let prompt = format!(
            "Are you sure you want to yank {crate_name}#{version} ({})?",
            v.id
        );
        if !dialoguer::confirm(&prompt) {
            return;
        }
    }

    println!("yanking version {} ({})", v.num, v.id);
    diesel::update(&v)
        .set(versions::yanked.eq(true))
        .execute(conn)
        .unwrap();

    crate::worker::sync_yanked(krate.name, v.num)
        .enqueue(conn)
        .unwrap();
}
