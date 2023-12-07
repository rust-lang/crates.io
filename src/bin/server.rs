#[macro_use]
extern crate tracing;

use crates_io::middleware::normalize_path::normalize_path;
use crates_io::{metrics::LogEncoder, util::errors::AppResult, App, Emails};
use std::{sync::Arc, time::Duration};

use axum::extract::Request;
use axum::ServiceExt;
use crates_io::github::RealGitHubClient;
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use prometheus::Encoder;
use reqwest::Client;
use std::io::Write;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::watch;
use tower::{Layer, Service};

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

    let mut make_service = axum_router.into_make_service_with_connect_info::<SocketAddr>();

    // to understand the following implementation,
    // see https://github.com/tokio-rs/axum/blob/axum-v0.7.2/examples/graceful-shutdown/src/main.rs
    // and https://github.com/tokio-rs/axum/blob/axum-v0.7.2/examples/serve-with-hyper/src/main.rs

    // Block the main thread until the server has shutdown
    rt.block_on(async {
        // Create a `TcpListener` using tokio.
        let listener = TcpListener::bind((app.config.ip, app.config.port)).await?;

        let addr = listener.local_addr()?;

        // Do not change this line! Removing the line or changing its contents in any way will break
        // the test suite :)
        info!("Listening at http://{addr}");

        // Create a watch channel to track tasks that are handling connections and wait for them to
        // complete.
        let (close_tx, close_rx) = watch::channel(());

        // Continuously accept new connections.
        loop {
            let (socket, remote_addr) = tokio::select! {
                // Either accept a new connection...
                result = listener.accept() => {
                    result.unwrap()
                }
                // ...or wait to receive a shutdown signal and stop the accept loop.
                _ = shutdown_signal() => {
                    debug!("shutdown signal received, not accepting new connections");
                    break;
                }
            };

            debug!("connection {remote_addr} accepted");

            // We don't need to call `poll_ready` because `IntoMakeServiceWithConnectInfo` is always
            // ready.
            let tower_service = make_service.call(remote_addr).await.unwrap();

            // Clone the watch receiver and move it into the task.
            let close_rx = close_rx.clone();

            // Spawn a task to handle the connection. That way we can serve multiple connections
            // concurrently.
            tokio::spawn(async move {
                // Hyper has its own `AsyncRead` and `AsyncWrite` traits and doesn't use tokio.
                // `TokioIo` converts between them.
                let socket = TokioIo::new(socket);

                // Hyper also has its own `Service` trait and doesn't use tower. We can use
                // `hyper::service::service_fn` to create a hyper `Service` that calls our app through
                // `tower::Service::call`.
                let hyper_service =
                    hyper::service::service_fn(move |request: Request<Incoming>| {
                        // We have to clone `tower_service` because hyper's `Service` uses `&self` whereas
                        // tower's `Service` requires `&mut self`.
                        //
                        // We don't need to call `poll_ready` since `Router` is always ready.
                        tower_service.clone().call(request.map(axum::body::Body::new))
                    });

                // `hyper_util::server::conn::auto::Builder` supports both http1 and http2 but doesn't
                // support graceful so we have to use hyper directly and unfortunately pick between
                // http1 and http2.
                let conn = hyper::server::conn::http1::Builder::new()
                    .serve_connection(socket, hyper_service)
                    // `with_upgrades` is required for websockets.
                    .with_upgrades();

                // `graceful_shutdown` requires a pinned connection.
                let mut conn = std::pin::pin!(conn);

                loop {
                    tokio::select! {
                        // Poll the connection. This completes when the client has closed the
                        // connection, graceful shutdown has completed, or we encounter a TCP error.
                        result = conn.as_mut() => {
                            if let Err(err) = result {
                                debug!("failed to serve connection: {err:#}");
                            }
                            break;
                        }
                        // Start graceful shutdown when we receive a shutdown signal.
                        //
                        // We use a loop to continue polling the connection to allow requests to finish
                        // after starting graceful shutdown. Our `Router` has `TimeoutLayer` so
                        // requests will finish after at most 30 seconds.
                        _ = shutdown_signal() => {
                            debug!("shutdown signal received, starting graceful connection shutdown");
                            conn.as_mut().graceful_shutdown();
                        }
                    }
                }

                debug!("connection {remote_addr} closed");

                // Drop the watch receiver to signal to `main` that this task is done.
                drop(close_rx);
            });
        }

        info!("Starting graceful shutdown");

        // We only care about the watch receivers that were moved into the tasks so close the residual
        // receiver.
        drop(close_rx);

        // Close the listener to stop accepting new connections.
        drop(listener);

        // Wait for all tasks to complete.
        debug!("waiting for {} tasks to finish", close_tx.receiver_count());
        close_tx.closed().await;

        Ok::<(), anyhow::Error>(())
    })?;

    info!("Persisting remaining downloads counters");
    match app.downloads_counter.persist_all_shards(&app) {
        Ok(stats) => stats.log(),
        Err(err) => error!(?err, "downloads_counter error"),
    }

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
