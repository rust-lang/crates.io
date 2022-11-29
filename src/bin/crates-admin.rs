#![warn(clippy::all, rust_2018_idioms)]

use cargo_registry::admin::{
    delete_crate, delete_version, enqueue_job, git_import, migrate, normalize_index, populate,
    render_readmes, test_pagerduty, transfer_crates, upload_index, verify_token, yank_version,
};

#[derive(clap::Parser, Debug)]
#[command(name = "crates-admin")]
enum Command {
    DeleteCrate(delete_crate::Opts),
    DeleteVersion(delete_version::Opts),
    Populate(populate::Opts),
    RenderReadmes(render_readmes::Opts),
    TestPagerduty(test_pagerduty::Opts),
    TransferCrates(transfer_crates::Opts),
    VerifyToken(verify_token::Opts),
    Migrate(migrate::Opts),
    UploadIndex(upload_index::Opts),
    YankVersion(yank_version::Opts),
    GitImport(git_import::Opts),
    #[clap(subcommand)]
    EnqueueJob(enqueue_job::Command),
    NormalizeIndex(normalize_index::Opts),
}

fn main() -> anyhow::Result<()> {
    let _sentry = cargo_registry::sentry::init();

    // Initialize logging
    cargo_registry::util::tracing::init();

    use clap::Parser;

    let command = Command::parse();

    match command {
        Command::DeleteCrate(opts) => delete_crate::run(opts),
        Command::DeleteVersion(opts) => delete_version::run(opts),
        Command::Populate(opts) => populate::run(opts),
        Command::RenderReadmes(opts) => render_readmes::run(opts)?,
        Command::TestPagerduty(opts) => test_pagerduty::run(opts)?,
        Command::TransferCrates(opts) => transfer_crates::run(opts),
        Command::VerifyToken(opts) => verify_token::run(opts).unwrap(),
        Command::Migrate(opts) => migrate::run(opts)?,
        Command::UploadIndex(opts) => upload_index::run(opts)?,
        Command::YankVersion(opts) => yank_version::run(opts),
        Command::GitImport(opts) => git_import::run(opts)?,
        Command::EnqueueJob(command) => enqueue_job::run(command)?,
        Command::NormalizeIndex(opts) => normalize_index::run(opts)?,
    }

    Ok(())
}
