#![warn(clippy::all, rust_2018_idioms)]

use clap::Clap;

use cargo_registry::admin::test_pagerduty::{run, Opts};

fn main() {
    let opts: Opts = Opts::parse();
    run(opts).unwrap()
}
