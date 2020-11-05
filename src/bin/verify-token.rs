use cargo_registry::admin::verify_token::{run, Opts};

use clap::Clap;

fn main() {
    let opts: Opts = Opts::parse();
    run(opts).unwrap()
}
