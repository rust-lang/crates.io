use crate::{
    admin::dialoguer,
    db,
    models::{Crate, OwnerKind, User},
    schema::{crate_owners, crates, users},
};

use crate::tasks::spawn_blocking;
use diesel::prelude::*;

#[derive(clap::Parser, Debug)]
#[command(
    name = "transfer-crates",
    about = "Transfer all crates from one user to another."
)]
pub struct Opts {
    /// GitHub login of the "from" user
    from_user: String,
    /// GitHub login of the "to" user
    to_user: String,
}

pub async fn run(opts: Opts) -> anyhow::Result<()> {
    spawn_blocking(move || {
        let conn = &mut db::oneoff_connection()?;
        transfer(opts, conn)?;
        Ok(())
    })
    .await
}

fn transfer(opts: Opts, conn: &mut PgConnection) -> anyhow::Result<()> {
    let from: User = users::table
        .filter(users::gh_login.eq(opts.from_user))
        .first(conn)?;

    let to: User = users::table
        .filter(users::gh_login.eq(opts.to_user))
        .first(conn)?;

    if from.gh_id != to.gh_id {
        println!("====================================================");
        println!("WARNING");
        println!();
        println!("this may not be the same github user, different github IDs");
        println!();
        println!("from: {:?}", from.gh_id);
        println!("to:   {:?}", to.gh_id);

        if !dialoguer::confirm("continue?")? {
            return Ok(());
        }
    }

    let prompt = format!(
        "Are you sure you want to transfer crates from {} to {}?",
        from.gh_login, to.gh_login
    );
    if !dialoguer::confirm(&prompt)? {
        return Ok(());
    }

    let crate_owners = crate_owners::table
        .filter(crate_owners::owner_id.eq(from.id))
        .filter(crate_owners::owner_kind.eq(OwnerKind::User));
    let crates: Vec<Crate> = Crate::all()
        .filter(crates::id.eq_any(crate_owners.select(crate_owners::crate_id)))
        .load(conn)?;

    for krate in crates {
        let owners = krate.owners(conn)?;
        if owners.len() != 1 {
            println!("warning: not exactly one owner for {}", krate.name);
        }
    }

    if !dialoguer::confirm("commit?")? {
        return Ok(());
    }

    diesel::update(crate_owners)
        .set(crate_owners::owner_id.eq(to.id))
        .execute(conn)?;

    Ok(())
}
