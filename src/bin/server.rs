#![warn(clippy::all, rust_2018_idioms)]

#[macro_use]
extern crate tracing;

use crates_io::middleware::normalize_path::normalize_path;
use crates_io::{metrics::LogEncoder, util::errors::AppResult, App};
use std::{fs::File, process::Command, sync::Arc, time::Duration};

use axum::ServiceExt;
use futures_util::future::FutureExt;
use prometheus::Encoder;
use reqwest::blocking::Client;
use std::io::{self, Write};
use std::net::SocketAddr;
use tokio::signal::unix::{signal, SignalKind};
use tower::Layer;

const CORE_THREADS: usize = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _sentry = crates_io::sentry::init();

    // Initialize logging
    crates_io::util::tracing::init();

    let _span = info_span!("server.run");

    let config = crates_io::config::Server::default();
    let client = Client::new();
    let app = Arc::new(App::new(config, Some(client)));

    // Start the background thread periodically persisting download counts to the database.
    downloads_counter_thread(app.clone());

    // Start the background thread periodically logging instance metrics.
    log_instance_metrics_thread(app.clone());

    let axum_router = crates_io::build_handler(app.clone());

    // Apply the `normalize_path` middleware around the axum router
    let normalize_path = axum::middleware::from_fn(normalize_path);
    let axum_router = normalize_path.layer(axum_router);

    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all();
    builder.worker_threads(CORE_THREADS);
    if let Some(threads) = app.config.max_blocking_threads {
        builder.max_blocking_threads(threads);
    }

    let rt = builder.build().unwrap();

    let make_service = axum_router.into_make_service_with_connect_info::<SocketAddr>();

    let (addr, server) = rt.block_on(async {
        let socket_addr = (app.config.ip, app.config.port).into();
        let server = hyper::Server::bind(&socket_addr).serve(make_service);

        // When the user configures PORT=0 the operating system will allocate a random unused port.
        // This fetches that random port and uses it to display the correct url later.
        let addr = server.local_addr();

        let mut sig_int = signal(SignalKind::interrupt())?;
        let mut sig_term = signal(SignalKind::terminate())?;
        let server = server.with_graceful_shutdown(async move {
            // Wait for either signal
            tokio::select! {
                _ = sig_int.recv().fuse() => {},
                _ = sig_term.recv().fuse() => {},
            };

            info!("Starting graceful shutdown");
        });

        Ok::<_, io::Error>((addr, server))
    })?;

    // Do not change this line! Removing the line or changing its contents in any way will break
    // the test suite :)
    info!("Listening at http://{addr}");

    // Creating this file tells heroku to tell nginx that the application is ready
    // to receive traffic.
    if app.config.use_nginx_wrapper {
        let path = "/tmp/app-initialized";
        info!("Writing to {path}");
        File::create(path).unwrap();

        // Launch nginx via the Heroku nginx buildpack
        // `wait()` is never called on the child process, but it should be okay to leave a zombie
        // process around on shutdown when Heroku is tearing down the entire container anyway.
        Command::new("./script/start-web.sh")
            .spawn()
            .expect("Couldn't spawn nginx");
    }

    // Block the main thread until the server has shutdown
    rt.block_on(server)?;

    info!("Persisting remaining downloads counters");
    match app.downloads_counter.persist_all_shards(&app) {
        Ok(stats) => stats.log(),
        Err(err) => error!(?err, "downloads_counter error"),
    }

    info!("Server has gracefully shutdown!");
    Ok(())
}

fn downloads_counter_thread(app: Arc<App>) {
    let interval =
        app.config.downloads_persist_interval / app.downloads_counter.shards_count() as u32;

    std::thread::spawn(move || loop {
        std::thread::sleep(interval);

        match app.downloads_counter.persist_next_shard(&app) {
            Ok(stats) => stats.log(),
            Err(err) => error!(?err, "downloads_counter error"),
        }
    });
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

fn log_instance_metrics_inner(app: &App) -> AppResult<()> {
    let families = app.instance_metrics.gather(app)?;

    let mut stdout = std::io::stdout();
    LogEncoder::new().encode(&families, &mut stdout)?;
    stdout.flush()?;

    Ok(())
}
