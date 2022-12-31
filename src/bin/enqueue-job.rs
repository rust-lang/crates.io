#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use cargo_registry::admin::enqueue_job::{run, Command};
use clap::Parser;

fn main() -> Result<()> {
    let command = Command::parse();
    run(command)
}
