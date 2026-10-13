#[macro_use]
extern crate tracing;

use crates_io::config::SharedConfig;
use crates_io::util::tracing as telemetry;
use tracing_subscriber::util::SubscriberInitExt;

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

    // Log without tracing until the trace provider is built.
    let bootstrap = tracing::subscriber::set_default(telemetry::subscriber(None)?);
    telemetry::install_panic_hook();

    let config = SharedConfig::from_environment()?;
    let provider = telemetry::tracer_provider(&config);

    drop(bootstrap);
    telemetry::subscriber(provider.as_ref())?.init();

    let result = match Command::parse() {
        Command::Admin(command) => admin::run(config, command),
        Command::Server => server::run(config),
        Command::BackgroundWorker => background_worker::run(config),
    };

    // The global subscriber keeps the provider alive, so `Drop` never flushes it.
    if let Some(Err(error)) = provider.map(|provider| provider.shutdown()) {
        warn!("Failed to shut down OpenTelemetry tracing: {error:#}");
    }

    result
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Command::command().debug_assert();
}
