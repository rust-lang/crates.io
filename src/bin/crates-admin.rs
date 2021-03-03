#![warn(clippy::all, rust_2018_idioms)]

use cargo_registry::admin::{
    delete_crate, delete_version, populate, render_readmes, test_new_semver_release,
    test_pagerduty, transfer_crates, verify_token,
};

use clap::Clap;

#[derive(Clap, Debug)]
#[clap(name = "crates-admin")]
struct Opts {
    #[clap(subcommand)]
    command: SubCommand,
}

#[derive(Clap, Debug)]
enum SubCommand {
    DeleteCrate(delete_crate::Opts),
    DeleteVersion(delete_version::Opts),
    Populate(populate::Opts),
    RenderReadmes(render_readmes::Opts),
    TestNewSemverRelease(test_new_semver_release::Opts),
    TestPagerduty(test_pagerduty::Opts),
    TransferCrates(transfer_crates::Opts),
    VerifyToken(verify_token::Opts),
}

fn main() {
    let opts: Opts = Opts::parse();

    match opts.command {
        SubCommand::DeleteCrate(opts) => delete_crate::run(opts),
        SubCommand::DeleteVersion(opts) => delete_version::run(opts),
        SubCommand::Populate(opts) => populate::run(opts),
        SubCommand::RenderReadmes(opts) => render_readmes::run(opts),
        SubCommand::TestNewSemverRelease(opts) => test_new_semver_release::run(opts),
        SubCommand::TestPagerduty(opts) => test_pagerduty::run(opts).unwrap(),
        SubCommand::TransferCrates(opts) => transfer_crates::run(opts),
        SubCommand::VerifyToken(opts) => verify_token::run(opts).unwrap(),
    }
}
