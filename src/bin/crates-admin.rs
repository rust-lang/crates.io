#![warn(clippy::all, rust_2018_idioms)]

use cargo_registry::admin::{
    delete_crate, delete_version, migrate, populate, render_readmes, test_pagerduty,
    transfer_crates, verify_token,
};

#[derive(clap::Parser, Debug)]
#[clap(name = "crates-admin")]
struct Opts {
    #[clap(subcommand)]
    command: SubCommand,
}

#[derive(clap::Parser, Debug)]
enum SubCommand {
    DeleteCrate(delete_crate::Opts),
    DeleteVersion(delete_version::Opts),
    Populate(populate::Opts),
    RenderReadmes(render_readmes::Opts),
    TestPagerduty(test_pagerduty::Opts),
    TransferCrates(transfer_crates::Opts),
    VerifyToken(verify_token::Opts),
    Migrate(migrate::Opts),
}

fn main() -> anyhow::Result<()> {
    use clap::Parser;

    let opts: Opts = Opts::parse();

    match opts.command {
        SubCommand::DeleteCrate(opts) => delete_crate::run(opts),
        SubCommand::DeleteVersion(opts) => delete_version::run(opts),
        SubCommand::Populate(opts) => populate::run(opts),
        SubCommand::RenderReadmes(opts) => render_readmes::run(opts)?,
        SubCommand::TestPagerduty(opts) => test_pagerduty::run(opts)?,
        SubCommand::TransferCrates(opts) => transfer_crates::run(opts),
        SubCommand::VerifyToken(opts) => verify_token::run(opts).unwrap(),
        SubCommand::Migrate(opts) => migrate::run(opts)?,
    }

    Ok(())
}
