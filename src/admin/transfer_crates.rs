use crate::{
    admin::dialoguer,
    db,
    models::{Crate, OwnerKind, User},
    schema::{crate_owners, crates, users},
};
use std::process::exit;

use diesel::prelude::*;

#[derive(clap::Parser, Debug)]
#[clap(
    name = "transfer-crates",
    about = "Transfer all crates from one user to another."
)]
pub struct Opts {
    /// GitHub login of the "from" user
    from_user: String,
    /// GitHub login of the "to" user
    to_user: String,
}

pub fn run(opts: Opts) {
    let conn = db::connect_now().unwrap();
    conn.transaction::<_, diesel::result::Error, _>(|| {
        transfer(opts, &conn);
        Ok(())
    })
    .unwrap()
}

fn transfer(opts: Opts, conn: &PgConnection) {
    let from: User = users::table
        .filter(users::gh_login.eq(opts.from_user))
        .first(conn)
        .unwrap();

    let to: User = users::table
        .filter(users::gh_login.eq(opts.to_user))
        .first(conn)
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

    let prompt = format!(
        "Are you sure you want to transfer crates from {} to {}?",
        from.gh_login, to.gh_login
    );
    get_confirm(&prompt);

    let crate_owners = crate_owners::table
        .filter(crate_owners::owner_id.eq(from.id))
        .filter(crate_owners::owner_kind.eq(OwnerKind::User as i32));
    let crates: Vec<Crate> = Crate::all()
        .filter(crates::id.eq_any(crate_owners.select(crate_owners::crate_id)))
        .load(conn)
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
    if !dialoguer::confirm(msg) {
        exit(0);
    }
}
