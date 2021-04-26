#![warn(clippy::all, rust_2018_idioms)]
#![allow(unknown_lints)]

use cargo_registry::{App, Env};
use std::{borrow::Cow, fs::File, process::Command, sync::Arc, time::Duration};

use conduit_hyper::Service;
use futures_util::future::FutureExt;
use reqwest::blocking::Client;
use sentry::{ClientOptions, IntoDsn};
use tokio::io::AsyncWriteExt;
use tokio::signal::unix::{signal, SignalKind};

const CORE_THREADS: usize = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _sentry = dotenv::var("SENTRY_DSN_API")
        .ok()
        .into_dsn()
        .expect("SENTRY_DSN_API is not a valid Sentry DSN value")
        .map(|dsn| {
            let mut opts = ClientOptions::from(dsn);
            opts.environment = Some(
                dotenv::var("SENTRY_ENV_API")
                    .map(Cow::Owned)
                    .expect("SENTRY_ENV_API must be set when using SENTRY_DSN_API"),
            );

            opts.release = dotenv::var("HEROKU_SLUG_COMMIT").ok().map(Into::into);

            sentry::init(opts)
        });

    // Initialize logging
    tracing_subscriber::fmt::init();

    let config = cargo_registry::Config::default();
    let env = config.env;
    let client = Client::new();
    let app = Arc::new(App::new(config, Some(client)));

    // Start the background thread periodically persisting download counts to the database.
    downloads_counter_thread(app.clone());

    let handler = cargo_registry::build_handler(app.clone());

    let heroku = dotenv::var("HEROKU").is_ok();
    let fastboot = dotenv::var("USE_FASTBOOT").is_ok();
    let dev_docker = dotenv::var("DEV_DOCKER").is_ok();

    let ip = if dev_docker {
        [0, 0, 0, 0]
    } else {
        [127, 0, 0, 1]
    };
    let port = if heroku {
        8888
    } else {
        dotenv::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8888)
    };
    let threads = dotenv::var("SERVER_THREADS")
        .map(|s| s.parse().expect("SERVER_THREADS was not a valid number"))
        .unwrap_or_else(|_| {
            if env == Env::Development {
                5
            } else {
                // A large default because this can be easily changed via env and in production we
                // want the logging middleware to accurately record the start time.
                500
            }
        });

    println!("Booting with a hyper based server");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(CORE_THREADS)
        .max_blocking_threads(threads as usize)
        .build()
        .unwrap();

    let handler = Arc::new(conduit_hyper::BlockingHandler::new(handler));
    let make_service =
        hyper::service::make_service_fn(move |socket: &hyper::server::conn::AddrStream| {
            let addr = socket.remote_addr();
            let handler = handler.clone();
            async move { Service::from_blocking(handler, addr) }
        });

    let addr = (ip, port).into();
    #[allow(clippy::async_yields_async)]
    let server = rt.block_on(async { hyper::Server::bind(&addr).serve(make_service) });

    let mut sig_int = rt.block_on(async { signal(SignalKind::interrupt()) })?;
    let mut sig_term = rt.block_on(async { signal(SignalKind::terminate()) })?;

    let server = server.with_graceful_shutdown(async move {
        // Wait for either signal
        futures_util::select! {
            _ = sig_int.recv().fuse() => {},
            _ = sig_term.recv().fuse() => {},
        };
        let mut stdout = tokio::io::stdout();
        stdout.write_all(b"Starting graceful shutdown\n").await.ok();
    });

    let server = rt.spawn(async { server.await.unwrap() });

    println!("listening on port {}", port);

    // Creating this file tells heroku to tell nginx that the application is ready
    // to receive traffic.
    if heroku {
        let path = if fastboot {
            "/tmp/backend-initialized"
        } else {
            "/tmp/app-initialized"
        };
        println!("Writing to {}", path);
        File::create(path).unwrap();

        // Launch nginx via the Heroku nginx buildpack
        // `wait()` is never called on the child process, but it should be okay to leave a zombie
        // process around on shutdown when Heroku is tearing down the entire container anyway.
        Command::new("./script/start-web.sh")
            .spawn()
            .expect("Couldn't spawn nginx");
    }

    // Block the main thread until the server has shutdown
    rt.block_on(async { server.await.unwrap() });

    println!("Persisting remaining downloads counters");
    match app.downloads_counter.persist_all_shards(&app) {
        Ok(stats) => stats.log(),
        Err(err) => println!("downloads_counter error: {}", err),
    }

    println!("Server has gracefully shutdown!");
    Ok(())
}

fn downloads_counter_thread(app: Arc<App>) {
    let interval = Duration::from_millis(
        (app.config.downloads_persist_interval_ms / app.downloads_counter.shards_count()) as u64,
    );

    std::thread::spawn(move || loop {
        std::thread::sleep(interval);

        match app.downloads_counter.persist_next_shard(&app) {
            Ok(stats) => stats.log(),
            Err(err) => println!("downloads_counter error: {}", err),
        }
    });
}
