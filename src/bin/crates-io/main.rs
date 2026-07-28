#[macro_use]
extern crate tracing;

mod admin;
mod background_worker;
mod monitor;
mod server;

#[derive(clap::Parser, Debug)]
#[command(name = "crates-io")]
enum Command {
    #[command(flatten)]
    Admin(admin::Command),
    Server,
    BackgroundWorker,
    Monitor,
}

fn main() -> anyhow::Result<()> {
    use clap::Parser;

    match Command::parse() {
        Command::Admin(command) => admin::run(command),
        Command::Server => server::run(),
        Command::BackgroundWorker => background_worker::run(),
        Command::Monitor => monitor::run(),
    }
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Command::command().debug_assert();
}
