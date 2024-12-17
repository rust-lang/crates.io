#[macro_use]
extern crate tracing;

use crates_io::middleware::normalize_path::normalize_path;
use crates_io::{metrics::LogEncoder, App, Emails};
use std::{sync::Arc, time::Duration};

use axum::ServiceExt;
use crates_io_github::RealGitHubClient;
use prometheus::Encoder;
use reqwest::Client;
use std::io::Write;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal::unix::{signal, SignalKind};
use tower::Layer;

const CORE_THREADS: usize = 4;

fn main() -> anyhow::Result<()> {
    let _sentry = crates_io::sentry::init();

    // Initialize logging
    crates_io::util::tracing::init();

    let _span = info_span!("server.run");

    let config = crates_io::config::Server::from_environment()?;

    let emails = Emails::from_environment(&config);

    let client = Client::new();
    let github = RealGitHubClient::new(client);
    let github = Box::new(github);

    let app = Arc::new(App::new(config, emails, github));

    // Start the background thread periodically logging instance metrics.
    log_instance_metrics_thread(app.clone());

    let axum_router = crates_io::build_handler(app.clone());

    // Apply the `normalize_path` middleware around the axum router.
    //
    // See https://docs.rs/axum/0.7.2/axum/middleware/index.html#rewriting-request-uri-in-middleware.
    let normalize_path = axum::middleware::from_fn(normalize_path);
    let axum_router = normalize_path.layer(axum_router);

    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all();
    builder.worker_threads(CORE_THREADS);
    if let Some(threads) = app.config.max_blocking_threads {
        builder.max_blocking_threads(threads);
    }

    let rt = builder.build()?;

    let make_service = axum_router.into_make_service_with_connect_info::<SocketAddr>();

    // Block the main thread until the server has shutdown
    rt.block_on(async {
        // Create a `TcpListener` using tokio.
        let listener = TcpListener::bind((app.config.ip, app.config.port)).await?;

        let addr = listener.local_addr()?;

        // Do not change this line! Removing the line or changing its contents in any way will break
        // the test suite :)
        info!("Listening at http://{addr}");

        // Run the server with graceful shutdown
        axum::serve(listener, make_service)
            .with_graceful_shutdown(shutdown_signal())
            .await
    })?;

    info!("Server has gracefully shutdown!");
    Ok(())
}

async fn shutdown_signal() {
    let interrupt = async {
        signal(SignalKind::interrupt())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    let terminate = async {
        signal(SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = interrupt => {},
        _ = terminate => {},
    }
}

fn log_instance_metrics_thread(app: Arc<App>) {
    // Only run the thread if the configuration is provided
    let interval = match app.config.instance_metrics_log_every_seconds {
        Some(secs) => Duration::from_secs(secs),
        None => return,
    };

    std::thread::spawn(move || loop {
        if let Err(err) = log_instance_metrics_inner(&app) {
            error!(?err, "log_instance_metrics error");
        }
        std::thread::sleep(interval);
    });
}

fn log_instance_metrics_inner(app: &App) -> anyhow::Result<()> {
    let families = app.instance_metrics.gather(app)?;

    let mut stdout = std::io::stdout();
    LogEncoder::new().encode(&families, &mut stdout)?;
    stdout.flush()?;

    Ok(())
}
