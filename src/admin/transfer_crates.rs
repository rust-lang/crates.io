use crate::{
    admin::dialoguer,
    db,
    models::{Crate, OwnerKind, User},
    schema::{crate_owners, crates, users},
};

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

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
    let mut conn = db::oneoff_async_connection().await?;
    transfer(opts, &mut conn).await?;
    Ok(())
}

async fn transfer(opts: Opts, conn: &mut AsyncPgConnection) -> anyhow::Result<()> {
    let from: User = users::table
        .filter(users::gh_login.eq(opts.from_user))
        .first(conn)
        .await?;

    let to: User = users::table
        .filter(users::gh_login.eq(opts.to_user))
        .first(conn)
        .await?;

    if from.gh_id != to.gh_id {
        println!("====================================================");
        println!("WARNING");
        println!();
        println!("this may not be the same github user, different github IDs");
        println!();
        println!("from: {:?}", from.gh_id);
        println!("to:   {:?}", to.gh_id);

        if !dialoguer::async_confirm("continue?").await? {
            return Ok(());
        }
    }

    let prompt = format!(
        "Are you sure you want to transfer crates from {} to {}?",
        from.gh_login, to.gh_login
    );
    if !dialoguer::async_confirm(&prompt).await? {
        return Ok(());
    }

    let crate_owners = crate_owners::table
        .filter(crate_owners::owner_id.eq(from.id))
        .filter(crate_owners::owner_kind.eq(OwnerKind::User));
    let crates: Vec<Crate> = Crate::all()
        .filter(crates::id.eq_any(crate_owners.select(crate_owners::crate_id)))
        .load(conn)
        .await?;

    for krate in crates {
        let num_owners: i64 = crate_owners::table
            .count()
            .filter(crate_owners::deleted.eq(false))
            .filter(crate_owners::crate_id.eq(krate.id))
            .get_result(conn)
            .await?;

        if num_owners != 1 {
            println!("warning: not exactly one owner for {}", krate.name);
        }
    }

    if !dialoguer::async_confirm("commit?").await? {
        return Ok(());
    }

    diesel::update(crate_owners)
        .set(crate_owners::owner_id.eq(to.id))
        .execute(conn)
        .await?;

    Ok(())
}
