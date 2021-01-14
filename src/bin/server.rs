#![warn(clippy::all, rust_2018_idioms)]
#![allow(clippy::unknown_clippy_lints)]

use cargo_registry::{boot, App, Env};
use std::{
    borrow::Cow,
    fs::File,
    sync::{mpsc::channel, Arc, Mutex},
    thread,
    time::Duration,
};

use civet::Server as CivetServer;
use conduit_hyper::Service;
use futures_util::future::FutureExt;
use reqwest::blocking::Client;
use sentry::{ClientOptions, IntoDsn};

const CORE_THREADS: usize = 4;

#[allow(clippy::large_enum_variant)]
enum Server {
    Civet(CivetServer),
    Hyper(tokio::runtime::Runtime, tokio::task::JoinHandle<()>),
}

use Server::*;

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
    let client = Client::new();

    let app = App::new(config.clone(), Some(client));
    let app = cargo_registry::build_handler(Arc::new(app));

    // On every server restart, ensure the categories available in the database match
    // the information in *src/categories.toml*.
    let categories_toml = include_str!("../boot/categories.toml");
    boot::categories::sync(categories_toml).unwrap();

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
            if config.env == Env::Development {
                5
            } else {
                // A large default because this can be easily changed via env and in production we
                // want the logging middleware to accurately record the start time.
                500
            }
        });

    let server = if dotenv::var("WEB_USE_CIVET").is_err() {
        use tokio::io::AsyncWriteExt;
        use tokio::signal::unix::{signal, SignalKind};

        println!("Booting with a hyper based server");

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(CORE_THREADS)
            .max_blocking_threads(threads as usize)
            .build()
            .unwrap();

        let handler = Arc::new(conduit_hyper::BlockingHandler::new(app));
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
        Hyper(rt, server)
    } else {
        println!("Booting with a civet based server");
        let mut cfg = civet::Config::new();
        cfg.port(port).threads(threads).keep_alive(true);
        Civet(CivetServer::start(cfg, app).unwrap())
    };

    println!("listening on port {}", port);

    // Give tokio a chance to spawn the first worker thread
    thread::sleep(Duration::from_millis(10));

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
    }

    // Block the main thread until the server has shutdown
    match server {
        Hyper(rt, server) => {
            rt.block_on(async { server.await.unwrap() });
        }
        Civet(server) => {
            let (tx, rx) = channel::<()>();
            ctrlc_handler(move || tx.send(()).unwrap_or(()));
            rx.recv().unwrap();
            drop(server);
        }
    }

    println!("Server has gracefully shutdown!");
    Ok(())
}

fn ctrlc_handler<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    let call_once = Mutex::new(Some(f));

    ctrlc::set_handler(move || {
        if let Some(f) = call_once.lock().unwrap().take() {
            println!("Starting graceful shutdown");
            f();
        } else {
            println!("Already sent signal to start graceful shutdown");
        }
    })
    .unwrap();
}
