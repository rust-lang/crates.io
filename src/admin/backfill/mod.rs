mod rust_version;

use self::rust_version::RustVersionOptions;

#[derive(clap::Parser, Debug)]
#[command(about = "Tools to backfill the database from various sources")]
pub enum Command {
    RustVersion(RustVersionOptions),
}

pub fn run(command: Command) -> anyhow::Result<()> {
    match command {
        Command::RustVersion(options) => rust_version::run(&options),
    }
}
