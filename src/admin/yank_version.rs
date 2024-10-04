use crate::admin::dialoguer;
use crate::db;
use crate::models::{Crate, Version};
use crate::schema::versions;
use crate::tasks::spawn_blocking;
use crate::worker::jobs;
use crate::worker::jobs::UpdateDefaultVersion;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;

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

pub async fn run(opts: Opts) -> anyhow::Result<()> {
    spawn_blocking(move || {
        let mut conn = db::oneoff_connection()?;
        conn.transaction(|conn| yank(opts, conn))?;
        Ok(())
    })
    .await
}

fn yank(opts: Opts, conn: &mut PgConnection) -> anyhow::Result<()> {
    let Opts {
        crate_name,
        version,
        yes,
    } = opts;
    let krate: Crate = Crate::by_name(&crate_name).first(conn)?;
    let v: Version = Version::belonging_to(&krate)
        .filter(versions::num.eq(&version))
        .first(conn)?;

    if v.yanked {
        println!("Version {version} of crate {crate_name} is already yanked");
        return Ok(());
    }

    if !yes {
        let prompt = format!(
            "Are you sure you want to yank {crate_name}#{version} ({})?",
            v.id
        );
        if !dialoguer::confirm(&prompt)? {
            return Ok(());
        }
    }

    println!("yanking version {} ({})", v.num, v.id);
    diesel::update(&v)
        .set(versions::yanked.eq(true))
        .execute(conn)?;

    jobs::enqueue_sync_to_index(&krate.name, conn)?;

    UpdateDefaultVersion::new(krate.id).enqueue(conn)?;

    Ok(())
}
