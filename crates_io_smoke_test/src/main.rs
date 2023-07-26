#[macro_use]
extern crate tracing;

use anyhow::Context;
use secrecy::SecretString;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

fn main() -> anyhow::Result<()> {
    init_tracing();

    let _token: SecretString = std::env::var("CARGO_REGISTRY_TOKEN")
        .context("Failed to read CARGO_REGISTRY_TOKEN environment variable")?
        .into();

    info!("Hello world!");

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
