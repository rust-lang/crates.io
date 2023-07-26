#[macro_use]
extern crate tracing;

use anyhow::{anyhow, Context};
use clap::Parser;
use secrecy::SecretString;
use std::process::Command;
use tempfile::tempdir;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

#[derive(clap::Parser, Debug)]
struct Options {
    /// name of the test crate that will be published to staging.crates.io
    #[arg(long, default_value = "crates-staging-test-tb")]
    crate_name: String,

    /// staging.crates.io API token that will be used to publish a new version
    #[arg(long, env = "CARGO_REGISTRY_TOKEN", hide_env_values = true)]
    token: SecretString,
}

fn main() -> anyhow::Result<()> {
    init_tracing();

    let options = Options::parse();
    debug!(?options);

    info!("Creating temporary working folder…");
    let tempdir = tempdir().context("Failed to create temporary working folder")?;
    debug!(tempdir.path = %tempdir.path().display());

    info!("Creating `{}` project…", options.crate_name);
    let exit_status = Command::new("cargo")
        .args(["new", "--lib", &options.crate_name])
        .current_dir(tempdir.path())
        .env("CARGO_TERM_COLOR", "always")
        .status()
        .context("Failed to run `cargo new`")?;

    if !exit_status.success() {
        return Err(anyhow!("Failed to run `cargo new`"));
    }

    let project_path = tempdir.path().join(&options.crate_name);
    debug!(project_path = %project_path.display());

    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let log_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_filter(env_filter);

    tracing_subscriber::registry().with(log_layer).init();
}
