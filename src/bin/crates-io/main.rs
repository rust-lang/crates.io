#[macro_use]
extern crate tracing;

use crates_io::config::SharedConfig;

mod admin;
mod background_worker;
mod server;

#[derive(clap::Parser, Debug)]
#[command(name = "crates-io")]
enum Command {
    #[command(flatten)]
    Admin(admin::Command),
    Server,
    BackgroundWorker,
}

fn main() -> anyhow::Result<()> {
    use clap::Parser;

    let _sentry = crates_io::sentry::init();

    // Initialize logging
    crates_io::util::tracing::init()?;

    let config = SharedConfig::from_environment()?;

    match Command::parse() {
        Command::Admin(command) => admin::run(config, command),
        Command::Server => server::run(config),
        Command::BackgroundWorker => background_worker::run(config),
    }
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Command::command().debug_assert();
}
