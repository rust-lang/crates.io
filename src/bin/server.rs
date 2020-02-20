#![warn(clippy::all, rust_2018_idioms)]

use cargo_registry::{boot, App, Env};
use std::{
    fs::File,
    sync::{mpsc::channel, Arc, Mutex},
    thread,
    time::Duration,
};

use civet::Server as CivetServer;
use conduit_hyper::Service;
use futures::prelude::*;
use reqwest::blocking::Client;

enum Server {
    Civet(CivetServer),
    Hyper(tokio::runtime::Runtime, tokio::task::JoinHandle<()>),
}

use Server::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    let config = cargo_registry::Config::default();
    let client = Client::new();

    let app = App::new(&config, Some(client));
    let app = cargo_registry::build_handler(Arc::new(app));

    // On every server restart, ensure the categories available in the database match
    // the information in *src/categories.toml*.
    let categories_toml = include_str!("../boot/categories.toml");
    boot::categories::sync(categories_toml).unwrap();

    let heroku = dotenv::var("HEROKU").is_ok();
    let fastboot = dotenv::var("USE_FASTBOOT").is_ok();

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
                1
            } else {
                50
            }
        });

    let server = if dotenv::var("USE_HYPER").is_ok() {
        use tokio::io::AsyncWriteExt;
        use tokio::signal::unix::{signal, SignalKind};

        println!("Booting with a hyper based server");

        let mut rt = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap();

        let handler = Arc::new(conduit_hyper::BlockingHandler::new(app, threads as usize));
        let make_service =
            hyper::service::make_service_fn(move |socket: &hyper::server::conn::AddrStream| {
                let addr = socket.remote_addr();
                let handler = handler.clone();
                async move { Service::from_blocking(handler, addr) }
            });

        let addr = ([127, 0, 0, 1], port).into();
        let server = rt.block_on(async { hyper::Server::bind(&addr).serve(make_service) });

        let mut sig_int = rt.block_on(async { signal(SignalKind::interrupt()) })?;
        let mut sig_term = rt.block_on(async { signal(SignalKind::terminate()) })?;

        let server = server.with_graceful_shutdown(async move {
            // Wait for either signal
            futures::select! {
                _ = sig_int.recv().fuse() => (),
                _ = sig_term.recv().fuse() => (),
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
        Hyper(mut rt, server) => {
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
