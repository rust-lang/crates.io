#![deny(warnings, clippy::all, rust_2018_idioms)]

use cargo_registry::{boot, App, Env};
use std::{
    fs::File,
    sync::{mpsc::channel, Arc, Mutex},
    thread,
    time::Duration,
};

use civet::Server as CivetServer;
use conduit_hyper::Service as HyperService;
use futures::Future;
use jemalloc_ctl;
use reqwest::Client;

enum Server {
    Civet(CivetServer),
    Hyper(tokio::runtime::Runtime),
}

use Server::*;

fn main() {
    let _ = jemalloc_ctl::set_background_thread(true);

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
        println!("Booting with a hyper based server");
        let addr = ([127, 0, 0, 1], port).into();
        let service = HyperService::new(app, threads as usize);
        let server = hyper::Server::bind(&addr).serve(service);

        let (tx, rx) = futures::sync::oneshot::channel::<()>();
        let server = server
            .with_graceful_shutdown(rx)
            .map_err(|e| log::error!("Server error: {}", e));

        ctrlc_handler(move || tx.send(()).unwrap_or(()));

        let mut rt = tokio::runtime::Builder::new()
            .core_threads(4)
            .name_prefix("hyper-server-worker-")
            .after_start(|| {
                log::debug!("Stared thread {}", thread::current().name().unwrap_or("?"))
            })
            .before_stop(|| {
                log::debug!(
                    "Stopping thread {}",
                    thread::current().name().unwrap_or("?")
                )
            })
            .build()
            .unwrap();
        rt.spawn(server);

        Hyper(rt)
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
        println!("Writing to /tmp/app-initialized");
        File::create("/tmp/app-initialized").unwrap();
    }

    // Block the main thread until the server has shutdown
    match server {
        Hyper(rt) => rt.shutdown_on_idle().wait().unwrap(),
        Civet(server) => {
            let (tx, rx) = channel::<()>();
            ctrlc_handler(move || tx.send(()).unwrap_or(()));
            rx.recv().unwrap();
            drop(server);
        }
    }

    println!("Server has gracefully shutdown!");
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
