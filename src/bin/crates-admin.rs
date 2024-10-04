#[macro_use]
extern crate tracing;

use crates_io::admin::{
    default_versions, delete_crate, delete_version, enqueue_job, migrate, populate, render_readmes,
    test_pagerduty, transfer_crates, upload_index, verify_token, yank_version,
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
    #[clap(subcommand)]
    EnqueueJob(enqueue_job::Command),
    #[clap(subcommand)]
    DefaultVersions(default_versions::Command),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _sentry = crates_io::sentry::init();

    // Initialize logging
    crates_io::util::tracing::init();

    use clap::Parser;

    let span = info_span!("admin.command", command = tracing::field::Empty);
    let command = Command::parse();
    span.record("command", tracing::field::debug(&command));

    match command {
        Command::DeleteCrate(opts) => delete_crate::run(opts).await,
        Command::DeleteVersion(opts) => delete_version::run(opts).await,
        Command::Populate(opts) => populate::run(opts).await,
        Command::RenderReadmes(opts) => render_readmes::run(opts).await,
        Command::TestPagerduty(opts) => test_pagerduty::run(opts).await,
        Command::TransferCrates(opts) => transfer_crates::run(opts).await,
        Command::VerifyToken(opts) => verify_token::run(opts).await,
        Command::Migrate(opts) => migrate::run(opts).await,
        Command::UploadIndex(opts) => upload_index::run(opts).await,
        Command::YankVersion(opts) => yank_version::run(opts).await,
        Command::EnqueueJob(command) => enqueue_job::run(command).await,
        Command::DefaultVersions(opts) => default_versions::run(opts).await,
    }
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Command::command().debug_assert();
}
