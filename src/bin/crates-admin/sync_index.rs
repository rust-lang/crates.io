use anyhow::Result;
use clap::builder::ArgAction;
use crates_io::db;
use crates_io::worker::jobs;
use crates_io_worker::BackgroundJob;

#[derive(clap::Parser, Debug)]
#[command(
    name = "sync-index",
    about = "Synchronize crate index data to git and sparse indexes"
)]
pub struct Opts {
    /// Name of the crate to synchronize
    name: String,

    /// Skip syncing to the git index
    #[arg(long = "no-git", action = ArgAction::SetFalse)]
    git: bool,

    /// Skip syncing to the sparse index
    #[arg(long = "no-sparse", action = ArgAction::SetFalse)]
    sparse: bool,
}

pub async fn run(opts: Opts) -> Result<()> {
    let mut conn = db::oneoff_connection().await?;

    if opts.git {
        println!("Enqueueing SyncToGitIndex job for `{}`", opts.name);
        jobs::SyncToGitIndex::new(&opts.name)
            .enqueue(&mut conn)
            .await?;
    }

    if opts.sparse {
        println!("Enqueueing SyncToSparseIndex job for `{}`", opts.name);
        jobs::SyncToSparseIndex::new(&opts.name)
            .enqueue(&mut conn)
            .await?;
    }

    Ok(())
}
