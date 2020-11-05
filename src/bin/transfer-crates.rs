#![warn(clippy::all, rust_2018_idioms)]

use cargo_registry::admin::transfer_crates::{run, Opts};

use clap::Clap;

fn main() {
    let opts: Opts = Opts::parse();
    run(opts)
}
