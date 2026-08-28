use super::dialoguer;
use crates_io::db;
use crates_io::models::{Crate, Version};
use crates_io::schema::versions;
use crates_io::worker::jobs::{SyncToGitIndex, SyncToSparseIndex, UpdateDefaultVersion};
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

#[derive(clap::Parser, Debug)]
#[command(
    name = "yank-version",
    about = "Yank a crate from the database and index."
)]
pub struct Opts {
    /// Name of the crate
    crate_name: String,
    /// Version number that should be yanked or unyanked
    version: String,
    /// Undo a yank, putting a version back into the index
    #[arg(short, long)]
    undo: bool,
    /// Don't ask for confirmation: yes, we are sure. Best for scripting.
    #[arg(short, long)]
    yes: bool,
}

pub async fn run(opts: Opts) -> anyhow::Result<()> {
    let mut conn = db::oneoff_connection().await?;

    conn.transaction(async |conn| yank(opts, conn).await)
        .await?;

    Ok(())
}

async fn yank(opts: Opts, conn: &mut AsyncPgConnection) -> anyhow::Result<()> {
    let Opts {
        crate_name,
        version,
        undo,
        yes,
    } = opts;
    let krate: Crate = Crate::by_name(&crate_name).first(conn).await?;

    let v: Version = Version::belonging_to(&krate)
        .filter(versions::num.eq(&version))
        .select(Version::as_select())
        .first(conn)
        .await?;

    let verb = if undo { "unyank" } else { "yank" };

    if v.yanked != undo {
        println!("Version {version} of crate {crate_name} is already {verb}ed");
        return Ok(());
    }

    if !yes {
        let prompt = format!(
            "Are you sure you want to {verb} {crate_name}#{version} ({})?",
            v.id
        );
        if !dialoguer::confirm(&prompt).await? {
            return Ok(());
        }
    }

    println!("{verb}ing version {} ({})", v.num, v.id);
    diesel::update(&v)
        .set(versions::yanked.eq(!undo))
        .execute(conn)
        .await?;

    let git_index_job = SyncToGitIndex::new(&krate.name);
    let sparse_index_job = SyncToSparseIndex::new(&krate.name);
    let update_default_version_job = UpdateDefaultVersion::new(krate.id);

    tokio::try_join!(
        git_index_job.enqueue(&*conn),
        sparse_index_job.enqueue(&*conn),
        update_default_version_job.enqueue(&*conn),
    )?;

    Ok(())
}
