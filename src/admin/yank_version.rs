use crate::admin::dialoguer;
use crate::db;
use crate::models::{Crate, Version};
use crate::schema::versions;
use crate::tasks::spawn_blocking;
use crate::worker::jobs;
use crate::worker::jobs::UpdateDefaultVersion;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

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
    let mut conn = db::oneoff_async_connection().await?;
    conn.transaction(|conn| yank(opts, conn))?;
    Ok(())
}

async fn yank(opts: Opts, conn: &mut AsyncPgConnection) -> anyhow::Result<()> {
    let Opts {
        crate_name,
        version,
        yes,
    } = opts;
    let krate: Crate = Crate::by_name(&crate_name).first(conn).await?;
    let v: Version = Version::belonging_to(&krate)
        .filter(versions::num.eq(&version))
        .first(conn)
        .await?;

    if v.yanked {
        println!("Version {version} of crate {crate_name} is already yanked");
        return Ok(());
    }

    if !yes {
        let prompt = format!(
            "Are you sure you want to yank {crate_name}#{version} ({})?",
            v.id
        );
        if !dialoguer::async_confirm(&prompt).await? {
            return Ok(());
        }
    }

    println!("yanking version {} ({})", v.num, v.id);
    diesel::update(&v)
        .set(versions::yanked.eq(true))
        .execute(conn)
        .await?;

    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        jobs::enqueue_sync_to_index(&krate.name, conn)?;

        UpdateDefaultVersion::new(krate.id).enqueue(conn)?;

        Ok(())
    })
    .await
}
